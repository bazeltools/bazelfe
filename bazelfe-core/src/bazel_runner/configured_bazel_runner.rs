use std::collections::HashMap;

use crate::build_events::build_event_server::BuildEventAction;
use crate::build_events::hydrated_stream::HydratedInfo;
use crate::buildozer_driver;

use crate::{
    bazel_runner,
    hydrated_stream_processors::{
        event_stream_listener::EventStreamListener,
        process_bazel_failures::{ProcessBazelFailures, TargetStory, TargetStoryAction},
    },
};
use crate::{build_events::build_event_server::bazel_event, config::Config};

use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use super::processor_activity::*;

pub struct ConfiguredBazel {
    sender_arc:
        Arc<Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>>,
    aes: EventStreamListener,
    bes_port: u16,
}

impl ConfiguredBazel {
    pub fn new(
        sender_arc: &Arc<
            Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
        >,
        aes: &EventStreamListener,
        bes_port: u16,
    ) -> Self {
        Self {
            sender_arc: sender_arc.clone(),
            aes: aes.clone(),
            bes_port,
        }
    }

    async fn spawn_bazel_attempt(
        &self,
        passthrough_args: &Vec<String>,
    ) -> (ProcessorActivity, bazel_runner::ExecuteResult) {
        spawn_bazel_attempt(&self.sender_arc, &self.aes, self.bes_port, passthrough_args).await
    }
}

async fn spawn_bazel_attempt(
    sender_arc: &Arc<
        Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
    >,
    aes: &EventStreamListener,
    bes_port: u16,
    passthrough_args: &Vec<String>,
) -> (ProcessorActivity, bazel_runner::ExecuteResult) {
    let (tx, rx) = async_channel::unbounded();
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };
    let error_stream = HydratedInfo::build_transformer(rx);

    let target_extracted_stream = aes.handle_stream(error_stream);

    let results_data = Arc::new(RwLock::new(None));
    let r_data = Arc::clone(&results_data);
    let recv_task = tokio::spawn(async move {
        let mut guard = r_data.write().await;

        let mut jvm_segments_indexed = 0;
        let mut actions_taken: u32 = 0;
        let mut target_story_actions = HashMap::new();

        while let Ok(action) = target_extracted_stream.recv().await {
            match action {
                crate::hydrated_stream_processors::BuildEventResponse::ProcessedBuildFailures(
                    pbf,
                ) => {
                    let current_updates: u32 = pbf
                        .target_story_entries
                        .iter()
                        .map(|e| match e.action {
                            TargetStoryAction::Success => 0,
                            _ => 1,
                        })
                        .sum();
                    actions_taken += current_updates;
                    for story_entry in pbf.target_story_entries {
                        match target_story_actions.get_mut(&story_entry.target) {
                            None => {
                                target_story_actions
                                    .insert(story_entry.target.clone(), vec![story_entry]);
                            }
                            Some(existing) => existing.push(story_entry),
                        };
                    }
                }
                crate::hydrated_stream_processors::BuildEventResponse::IndexedResults(ir) => {
                    jvm_segments_indexed += ir.jvm_segments_indexed
                }
            }
        }

        *guard = Some(ProcessorActivity {
            jvm_segments_indexed: jvm_segments_indexed,
            actions_taken: actions_taken,
            target_story_actions: target_story_actions,
        });
    });

    let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;

    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };

    recv_task.await.unwrap();
    let r = results_data.write().await.take().unwrap();
    (r, res)
}

pub struct ConfiguredBazelRunner<
    T: buildozer_driver::Buildozer,
    U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
> {
    config: Arc<Config>,
    configured_bazel: ConfiguredBazel,
    runner_daemon: Option<crate::bazel_runner_daemon::daemon_service::RunnerDaemonClient>,
    _index_table: crate::index_table::IndexTable,
    _aes: EventStreamListener,
    passthrough_args: Vec<String>,
    process_build_failures: Arc<ProcessBazelFailures<T, U>>,
}

impl<
        T: buildozer_driver::Buildozer,
        U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
    > ConfiguredBazelRunner<T, U>
{
    pub fn new(
        config: Arc<Config>,
        configured_bazel: ConfiguredBazel,
        runner_daemon: Option<crate::bazel_runner_daemon::daemon_service::RunnerDaemonClient>,
        index_table: crate::index_table::IndexTable,
        aes: EventStreamListener,
        passthrough_args: Vec<String>,
        process_build_failures: Arc<ProcessBazelFailures<T, U>>,
    ) -> Self {
        Self {
            config,
            configured_bazel,
            runner_daemon,
            _index_table: index_table,
            _aes: aes,
            passthrough_args,
            process_build_failures,
        }
    }
    pub async fn run(self) -> Result<i32, Box<dyn std::error::Error>> {
        let mut attempts: u16 = 0;

        let mut running_total = ProcessorActivity::default();
        let mut final_exit_code = 0;
        let disable_action_stories_on_success = self.config.disable_action_stories_on_success;
        let mut total_actions_taken: u32 = 0;
        while attempts < 15 {
            attempts += 1;
            self.process_build_failures.advance_epoch().await;

            let (processor_activity, bazel_result) = self
                .configured_bazel
                .spawn_bazel_attempt(&self.passthrough_args)
                .await;
            let actions_taken = processor_activity.actions_taken;
            total_actions_taken += actions_taken;
            running_total.merge(processor_activity, disable_action_stories_on_success);
            final_exit_code = bazel_result.exit_code;
            if bazel_result.exit_code == 0 || actions_taken == 0 {
                break;
            }
        }

        // we should be very quiet if the build is successful/we added nothing.
        if total_actions_taken > 0 && !(final_exit_code == 0 && disable_action_stories_on_success) {
            eprintln!("--------------------Bazel Runner Report--------------------");

            if running_total.target_story_actions.len() > 0 {
                if final_exit_code != 0 {
                    eprintln!(
                    "\nBuild still failed. Active stories about failed targets/what we've tried:"
                );
                } else {
                    eprintln!("\nBuild succeeded, but documenting actions we took(some may have failed, but the build completed ok.):\n");
                }
                let mut v: Vec<(String, Vec<TargetStory>)> =
                    running_total.target_story_actions.into_iter().collect();
                v.sort_by_key(|k| k.0.clone());
                for (label, mut story_entries) in v.into_iter() {
                    eprintln!("Target: {}", label);
                    story_entries.sort_by_key(|e| e.when.clone());
                    for entry in story_entries.into_iter() {
                        match entry.action {
                            TargetStoryAction::AddedDependency { added_what, why } => {
                                eprintln!("\tAdded Dependency {}\n\t\tReason: {}", added_what, why);
                            }
                            TargetStoryAction::RemovedDependency { removed_what, why } => {
                                eprintln!(
                                    "\tRemoved Dependency {}\n\t\tReason: {}",
                                    removed_what, why
                                );
                            }
                            TargetStoryAction::Success => eprintln!("\tTarget suceeded"),
                            TargetStoryAction::RanUserAction {
                                user_action_name,
                                why,
                                command_line,
                                execution_result,
                            } => {
                                if execution_result.exit_success {
                                    eprintln!("\tRan user action: {}\n\t\tReason: {}\n\t\tSuccess: {}\n\t\tCommand line: {}", user_action_name, why, true, command_line);
                                } else {
                                    eprintln!("\tRan user action: {}\n\t\tReason: {}\n\t\tSuccess: {}\n\t\tCommand line: {}\nstdout:\n{}\n\nstderr:\n{}\n\n", user_action_name, why, false, command_line, execution_result.stdout, execution_result.stderr);
                                }
                            }
                        }
                    }
                }
            }
            eprintln!("Bazel exit code: {}", final_exit_code);
            eprintln!("Bazel build attempts: {}", attempts);
            eprintln!("Actions taken: {}", running_total.actions_taken);
            eprintln!(
                "Jvm fragments (classes/packages) added to index: {}",
                running_total.jvm_segments_indexed
            );
            eprintln!("------------------------------------------------------------\n");
        }

        Ok(final_exit_code)
    }
}
