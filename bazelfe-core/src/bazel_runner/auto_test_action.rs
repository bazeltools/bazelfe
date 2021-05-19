use std::time::SystemTime;

use crate::bazel_command_line_parser::{BuiltInAction, ParsedCommandLine};
use crate::{
    bazel_command_line_parser::CustomAction, bazel_runner_daemon::daemon_service::FileStatus,
    buildozer_driver,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AutoTestActionError {
    #[error("Requested Autotest, but the daemon isn't running")]
    NoDaemon,
}

use super::configured_bazel_runner::ConfiguredBazelRunner;
fn current_ms_since_epoch() -> u128 {
    SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u128
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

        let mut invalid_since_when: u128 = current_ms_since_epoch() - 20000;
        let mut cur_distance = 1;
        let max_distance = 1;
        let mut dirty_files: Vec<FileStatus> = Vec::default();

        loop {
            let recent_changed_files: Vec<FileStatus> = daemon_cli
                .wait_for_files(tarpc::context::current(), invalid_since_when)
                .await?;
            if !recent_changed_files.is_empty() {
                invalid_since_when = recent_changed_files.iter().map(|e| e.1).max().unwrap();
                dirty_files.extend(recent_changed_files);
                let changed_targets = daemon_cli
                    .targets_from_files(
                        tarpc::context::current(),
                        dirty_files.clone(),
                        cur_distance,
                    )
                    .await?;

                configured_bazel_runner.bazel_command_line.action = Some(
                    crate::bazel_command_line_parser::Action::BuiltIn(BuiltInAction::Build),
                );
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

                let result = configured_bazel_runner.run_command_line().await?;
                if result.final_exit_code != 0 {
                    continue;
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
                    configured_bazel_runner.bazel_command_line.action = Some(
                        crate::bazel_command_line_parser::Action::BuiltIn(BuiltInAction::Build),
                    );

                    let result = configured_bazel_runner.run_command_line().await?;
                    if result.final_exit_code != 0 {
                        continue;
                    }
                }
                if cur_distance >= max_distance {
                    cur_distance = 1;
                    dirty_files.clear();
                } else {
                    cur_distance += 1;
                }
            }
        }
    }
    //     if bazel_command_line.remaining_args.is_empty() {
    //         match &command_line_rewriter.test {
    //             TestActionMode::EmptyTestToLocalRepo(cfg) => {
    //                 bazel_command_line
    //                     .remaining_args
    //                     .push(cfg.command_to_use.clone());
    //             }
    //             TestActionMode::EmptyTestToFail => {
    //                 Err(RewriteCommandLineError::UserErrorReport(super::UserReportError("No test target specified.\nUnlike other build tools, bazel requires you specify which test target to test.\nTo test the whole repo add //... to the end. But beware this could be slow!".to_owned())))?;
    //             }
    //             TestActionMode::Passthrough => {}
    //             TestActionMode::SuggestTestTarget(cfg) => {
    //                 if let Some(daemon_cli) = daemon_client.as_ref() {
    //                     let mut invalidated_targets = vec![];

    //                     for distance in 0..(cfg.distance_to_expand + 1) {
    //                         let recently_invalidated_targets = daemon_cli
    //                             .recently_invalidated_targets(tarpc::context::current(), distance)
    //                             .await;
    //                         invalidated_targets.extend(
    //                             recently_invalidated_targets
    //                                 .into_iter()
    //                                 .map(|e| (distance, e)),
    //                         );
    //                     }
    //                     if !invalidated_targets.is_empty() {
    //                         use trim_margin::MarginTrimmable;

    //                         invalidated_targets.sort_by_key(|e| e.0);
    //                         use std::collections::HashSet;
    //                         let mut seen_targets: HashSet<String> = HashSet::default();
    //                         let mut buf = String::from("");
    //                         invalidated_targets.into_iter().for_each(|(_, targets)| {
    //                             targets.iter().for_each(|target| {
    //                                 if target.is_test() {
    //                                     if !seen_targets.contains(target.target_label()) {
    //                                         seen_targets.insert(target.target_label().clone());
    //                                         buf.push_str(&format!("\n|{}", target.target_label()));
    //                                     }
    //                                 }
    //                             });
    //                         });

    //                         Err(RewriteCommandLineError::UserErrorReport(
    //                             super::UserReportError(
    //                                 format!(
    //                                     r#"|No test target specified.
    //                                 | Suggestions:
    //                                 |{}
    //                                 |"#,
    //                                     buf
    //                                 )
    //                                 .trim_margin()
    //                                 .unwrap()
    //                                 .to_owned(),
    //                             ),
    //                         ))?;
    //                     }
    //                 } else {
    //                     Err(RewriteCommandLineError::UserErrorReport(super::UserReportError(
    //                             "Configured to suggest possible test targets to run, but no daemon is running".to_owned())))?;
    //                 }
    //             }
    //         }
    //     }
    // }

    Ok(false)
}

#[cfg(test)]
mod tests {}
