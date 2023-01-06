use std::collections::HashMap;

use crate::buildozer_driver;
use crate::hydrated_stream_processors::BuildEventResponse;

use crate::config::Config;
use crate::hydrated_stream_processors::process_bazel_failures::{
    ProcessBazelFailures, TargetStory, TargetStoryAction,
};

use bazelfe_bazel_wrapper::bazel_command_line_parser::ParsedCommandLine;
use bazelfe_bazel_wrapper::bazel_subprocess_wrapper::BazelWrapper;
use bazelfe_bazel_wrapper::bazel_subprocess_wrapper::{BazelWrapperError, ExecuteResult};
use std::sync::Arc;

use tokio::sync::RwLock;

use super::processor_activity::*;

async fn run_bazel(
    configured_bazel: &BazelWrapper<BuildEventResponse>,
    bazel_command_line: &ParsedCommandLine,
    pipe_output: bool,
) -> Result<(ProcessorActivity, ExecuteResult), Box<dyn std::error::Error>> {
    let (tx, rx) = async_channel::unbounded();
    let results_data = Arc::new(RwLock::new(None));
    let r_data = Arc::clone(&results_data);
    let recv_task = tokio::spawn(async move {
        let mut guard = r_data.write().await;
        let mut jvm_segments_indexed = 0;
        let mut actions_taken: u32 = 0;
        let mut target_story_actions = HashMap::new();

        while let Ok(action) = rx.recv().await {
            match action {
                crate::hydrated_stream_processors::BuildEventResponse::ProcessedBuildFailures(
                    pbf,
                ) => {
                    let current_updates: u32 = pbf
                        .target_story_entries
                        .iter()
                        .map(|e| match e.action {
                            TargetStoryAction::Success => 0,
                            TargetStoryAction::WouldHaveAddedDependency { .. } => 0,
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
            jvm_segments_indexed,
            actions_taken,
            target_story_actions,
        });
    });

    let res = configured_bazel
        .spawn_bazel_attempt(bazel_command_line, pipe_output, tx)
        .await
        .map_err(|e| BazelWrapperError::Unknown(e))?;
    recv_task.await.unwrap();
    let r = results_data.write().await.take().unwrap();
    Ok((r, res))
}

pub struct ConfiguredBazelRunner<
    T: buildozer_driver::Buildozer,
    U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
> {
    config: Arc<Config>,
    pub configured_bazel: BazelWrapper<BuildEventResponse>,
    #[cfg(feature = "bazelfe-daemon")]
    pub runner_daemon: Option<
        bazelfe_protos::bazel_tools::daemon_service::daemon_service_client::DaemonServiceClient<
            tonic::transport::Channel,
        >,
    >,
    _index_table: crate::index_table::IndexTable,
    pub bazel_command_line: ParsedCommandLine,
    process_build_failures: Arc<ProcessBazelFailures<T, U>>,
}

impl<
        T: buildozer_driver::Buildozer,
        U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
    > ConfiguredBazelRunner<T, U>
{
    pub fn new(
        config: Arc<Config>,
        configured_bazel: BazelWrapper<BuildEventResponse>,
        #[cfg(feature = "bazelfe-daemon")] runner_daemon: Option<
            bazelfe_protos::bazel_tools::daemon_service::daemon_service_client::DaemonServiceClient<
                tonic::transport::Channel,
            >,
        >,
        index_table: crate::index_table::IndexTable,
        bazel_command_line: ParsedCommandLine,
        process_build_failures: Arc<ProcessBazelFailures<T, U>>,
    ) -> Self {
        Self {
            config,
            configured_bazel,
            #[cfg(feature = "bazelfe-daemon")]
            runner_daemon,
            _index_table: index_table,
            bazel_command_line,
            process_build_failures,
        }
    }

    pub async fn run_command_line(
        &self,
        pipe_output: bool,
    ) -> Result<RunCompleteState, Box<dyn std::error::Error>> {
        let mut attempts: u16 = 0;

        let mut running_total = ProcessorActivity::default();
        let mut final_exit_code = 0;
        let disable_action_stories_on_success = self.config.disable_action_stories_on_success;
        let mut total_actions_taken: u32 = 0;
        while attempts < 60 {
            attempts += 1;
            self.process_build_failures.advance_epoch().await;
            let (processor_activity, bazel_result) = run_bazel(
                &self.configured_bazel,
                &self.bazel_command_line,
                pipe_output,
            )
            .await?;
            let actions_taken = processor_activity.actions_taken;
            total_actions_taken += actions_taken;
            running_total.merge(processor_activity, disable_action_stories_on_success);
            final_exit_code = bazel_result.exit_code;
            if bazel_result.exit_code == 0 || actions_taken == 0 {
                break;
            }
        }
        Ok(RunCompleteState {
            attempts,
            total_actions_taken,
            final_exit_code,
            running_total,
        })
    }

    // todo, move me to the app, this is app specific
    pub async fn run(mut self) -> Result<i32, BazelWrapperError> {
        let bq = crate::jvm_indexer::bazel_query::from_binary_path(
            &self.bazel_command_line.bazel_binary,
        );
        super::command_line_rewriter_action::rewrite_command_line(
            &mut self.bazel_command_line,
            &self.config.command_line_rewriter,
            #[cfg(feature = "bazelfe-daemon")]
            &mut self.runner_daemon,
            bq,
        )
        .await
        .map_err(|e| BazelWrapperError::Unknown(Box::new(e)))?;

        #[cfg(feature = "autotest-action")]
        if super::auto_test_action::maybe_auto_test_mode(&mut self)
            .await
            .map_err(|e| BazelWrapperError::Unknown(e))?
        {
            return Ok(0);
        };
        let res_data = self
            .run_command_line(true)
            .await
            .map_err(|e| BazelWrapperError::Unknown(e))?;
        let disable_action_stories_on_success = self.config.disable_action_stories_on_success;

        // we should be very quiet if the build is successful/we added nothing.
        if res_data.total_actions_taken > 0
            && !(res_data.final_exit_code == 0 && disable_action_stories_on_success)
        {
            eprintln!("--------------------Bazel Runner Report--------------------");

            if !res_data.running_total.target_story_actions.is_empty() {
                if res_data.final_exit_code != 0 {
                    eprintln!(
                    "\nBuild still failed. Active stories about failed targets/what we've tried:"
                );
                } else {
                    eprintln!("\nBuild succeeded, but documenting actions we took(some may have failed, but the build completed ok.):\n");
                }
                let mut v: Vec<(String, Vec<TargetStory>)> = res_data
                    .running_total
                    .target_story_actions
                    .into_iter()
                    .collect();
                v.sort_by_key(|k| k.0.clone());
                for (label, mut story_entries) in v.into_iter() {
                    eprintln!("Target: {}", label);
                    story_entries.sort_by_key(|e| e.when);
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
                            TargetStoryAction::WouldHaveAddedDependency { what, why } => {
                                eprintln!(
                                    "\tWould have, but didn't Add Dependency {}\n\t\tReason: {}",
                                    what, why
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
                                    eprintln!("\tRan user action: {}\n\t\tReason: {}\n\t\tSuccess: true\n\t\tCommand line: {}", user_action_name, why, command_line);
                                } else {
                                    eprintln!("\tRan user action: {}\n\t\tReason: {}\n\t\tSuccess: false\n\t\tCommand line: {}\nstdout:\n{}\n\nstderr:\n{}\n\n", user_action_name, why, command_line, execution_result.stdout, execution_result.stderr);
                                }
                            }
                        }
                    }
                }
            }
            eprintln!("Bazel exit code: {}", res_data.final_exit_code);
            eprintln!("Bazel build attempts: {}", res_data.attempts);
            eprintln!("Actions taken: {}", res_data.running_total.actions_taken);
            eprintln!(
                "Jvm fragments (classes/packages) added to index: {}",
                res_data.running_total.jvm_segments_indexed
            );
            eprintln!("------------------------------------------------------------\n");
        }

        Ok(res_data.final_exit_code)
    }
}

pub struct RunCompleteState {
    pub attempts: u16,
    pub total_actions_taken: u32,
    pub final_exit_code: i32,
    pub running_total: ProcessorActivity,
}
