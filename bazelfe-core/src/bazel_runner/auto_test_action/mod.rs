mod app;
mod command_line_driver;
mod ctrl_char;
mod progress_tab_updater;
mod ui;
mod util;

use std::{collections::HashSet, path::PathBuf, sync::Arc, time::Instant};

use crate::bazel_command_line_parser::BuiltInAction;
use crate::{
    bazel_command_line_parser::CustomAction, bazel_runner_daemon::daemon_service::FileStatus,
    buildozer_driver,
};

use thiserror::Error;

#[derive(Debug, Clone, Copy)]
pub enum BazelStatus {
    Idle,
    InQuery,
    Build,
    Test,
}

#[derive(Debug, Clone, Copy)]
pub enum BuildStatus {
    ActionsGreen,
    ActionsFailing,
    Unknown,
}
#[derive(Error, Debug)]
pub enum AutoTestActionError {
    #[error("Requested Autotest, but the daemon isn't running")]
    NoDaemon,
}

use super::configured_bazel_runner::ConfiguredBazelRunner;

#[derive(Debug)]
pub enum CompleteKind {
    Action,
    Target,
    Test,
}
use bazelfe_protos::*;

#[derive(Debug)]
pub struct ActionTargetStateScrollEntry {
    pub complete_type: CompleteKind,
    pub success: bool,
    pub label: String,
    pub when: Instant,
    pub target_kind: Option<String>,
    pub bazel_run_id: usize,
    pub files: Vec<build_event_stream::File>,
}

pub async fn maybe_auto_test_mode<
    T: buildozer_driver::Buildozer,
    U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
>(
    configured_bazel_runner: &mut ConfiguredBazelRunner<T, U>,
) -> Result<bool, Box<dyn std::error::Error>> {
    if configured_bazel_runner.bazel_command_line.action
        == Some(crate::bazel_command_line_parser::Action::Custom(
            CustomAction::AutoTest,
        ))
    {
        configured_bazel_runner.bazel_command_line.action = Some(
            crate::bazel_command_line_parser::Action::BuiltIn(BuiltInAction::Test),
        );

        let daemon_cli = if let Some(daemon_cli) = configured_bazel_runner.runner_daemon.as_ref() {
            Ok(daemon_cli)
        } else {
            Err(AutoTestActionError::NoDaemon)
        }?;
        let (bazel_status_tx, bazel_status_rx) = flume::unbounded::<BazelStatus>();
        let (build_status_tx, build_status_rx) = flume::unbounded::<BuildStatus>();
        let (progress_pump_sender, progress_receiver) = flume::unbounded::<String>();
        let (changed_file_tx, changed_file_rx) = flume::unbounded::<Vec<(FileStatus, Instant)>>();
        let (action_event_tx, action_event_rx) = flume::unbounded::<ActionTargetStateScrollEntry>();

        let progress_tab_updater =
            progress_tab_updater::ProgressTabUpdater::new(progress_pump_sender, action_event_tx);

        configured_bazel_runner
            .configured_bazel
            .aes
            .add_event_handler(Arc::new(progress_tab_updater));

        let mut invalid_since_when: u128 = 0;
        let mut cur_distance = 1;
        let max_distance = 3;
        let mut dirty_files: Vec<(FileStatus, Instant)> = Vec::default();

        let main_running = command_line_driver::main(
            progress_receiver,
            changed_file_rx,
            action_event_rx,
            bazel_status_rx,
            build_status_rx,
        )?;
        let mut bazel_in_query = false;
        let mut successful_files: HashSet<FileStatus> = HashSet::default();
        'outer_loop: loop {
            dirty_files.retain(|(e, _)| !successful_files.contains(e));
            let _ = changed_file_tx.send_async(dirty_files.clone()).await;
            successful_files.clear();
            match main_running.try_recv() {
                Ok(inner_result) => {
                    if let Err(e) = inner_result {
                        eprintln!("UX system failed with: {}", e);
                        break 'outer_loop;
                    }
                }
                Err(e) => match e {
                    flume::TryRecvError::Empty => (),
                    flume::TryRecvError::Disconnected => {
                        break 'outer_loop;
                    }
                },
            }

            let recent_changed_files = daemon_cli
                .wait_for_files(tarpc::context::current(), invalid_since_when)
                .await?;

            if !recent_changed_files.is_empty() {
                invalid_since_when = daemon_cli
                    .request_instant(tarpc::context::current())
                    .await?;

                let mut visited_files: HashSet<PathBuf> = HashSet::default();
                let mut visited_targets: HashSet<String> = HashSet::default();

                let now = Instant::now();
                dirty_files.extend(recent_changed_files.into_iter().map(|e| (e, now)));
                dirty_files.sort_by_key(|(e, _)| e.1);
                dirty_files.reverse();

                let _ = changed_file_tx.send_async(dirty_files.clone()).await;

                'dirty_file_loop: for (f, _) in dirty_files.iter() {
                    if visited_files.contains(&f.0) {
                        continue 'dirty_file_loop;
                    }
                    visited_files.insert(f.0.clone());
                    'inner_loop: loop {
                        let changed_targets_resp = daemon_cli
                            .targets_from_files(
                                tarpc::context::current(),
                                vec![f.clone()],
                                cur_distance,
                                bazel_in_query,
                            )
                            .await?;

                        let mut changed_targets = match changed_targets_resp {
                            crate::bazel_runner_daemon::daemon_service::TargetsFromFilesResponse::InQuery => {
                                let _ = bazel_status_tx.send_async(BazelStatus::InQuery).await;
                                bazel_in_query = true;
                                continue 'inner_loop;
                            }
                            crate::bazel_runner_daemon::daemon_service::TargetsFromFilesResponse::Targets(t) => {
                                bazel_in_query = false;
                                            let _ = bazel_status_tx.send_async(BazelStatus::Idle).await;
                                            t
                            }
                        };

                        changed_targets.retain(|e| !visited_targets.contains(e.target_label()));
                        changed_targets.iter().for_each(|e| {
                            visited_targets.insert(e.target_label().clone());
                            ()
                        });

                        if !changed_targets.is_empty() {
                            configured_bazel_runner.bazel_command_line.action =
                                Some(crate::bazel_command_line_parser::Action::BuiltIn(
                                    BuiltInAction::Build,
                                ));
                            configured_bazel_runner
                                .bazel_command_line
                                .remaining_args
                                .clear();

                            for t in changed_targets.iter() {
                                configured_bazel_runner
                                    .bazel_command_line
                                    .remaining_args
                                    .push(t.target_label().clone());
                            }

                            let _ = bazel_status_tx.send_async(BazelStatus::Build).await;
                            let result = configured_bazel_runner.run_command_line(false).await?;
                            let _ = bazel_status_tx.send_async(BazelStatus::Idle).await;
                            if result.final_exit_code != 0 {
                                build_status_tx
                                    .send_async(BuildStatus::ActionsFailing)
                                    .await?;
                                continue 'outer_loop;
                            }

                            // Now try tests

                            configured_bazel_runner
                                .bazel_command_line
                                .remaining_args
                                .clear();

                            for t in changed_targets.iter() {
                                if t.is_test() {
                                    configured_bazel_runner
                                        .bazel_command_line
                                        .remaining_args
                                        .push(t.target_label().clone());
                                }
                            }

                            if !configured_bazel_runner
                                .bazel_command_line
                                .remaining_args
                                .is_empty()
                            {
                                configured_bazel_runner.bazel_command_line.action =
                                    Some(crate::bazel_command_line_parser::Action::BuiltIn(
                                        BuiltInAction::Test,
                                    ));

                                let _ = bazel_status_tx.send_async(BazelStatus::Test).await;
                                let result =
                                    configured_bazel_runner.run_command_line(false).await?;
                                let _ = bazel_status_tx.send_async(BazelStatus::Idle).await;

                                if result.final_exit_code != 0 {
                                    build_status_tx
                                        .send_async(BuildStatus::ActionsFailing)
                                        .await?;

                                    continue 'outer_loop;
                                }
                            }
                            build_status_tx
                                .send_async(BuildStatus::ActionsGreen)
                                .await?;
                        }
                        if cur_distance >= max_distance {
                            cur_distance = 1;
                            successful_files.insert(f.clone());
                            break 'inner_loop;
                        } else {
                            cur_distance += 1;
                        }
                    }
                }
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {}
