use crate::bazel_command_line_parser::BuiltInAction;
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

        let mut invalid_since_when: u128 = 0;
        let mut cur_distance = 1;
        let max_distance = 3;
        let mut dirty_files: Vec<FileStatus> = Vec::default();

        loop {
            let recent_changed_files: Vec<FileStatus> = daemon_cli
                .wait_for_files(tarpc::context::current(), invalid_since_when)
                .await?;
            if !recent_changed_files.is_empty() {
                eprintln!("Changed: {:#?}", recent_changed_files);
                invalid_since_when = recent_changed_files.iter().map(|e| e.1 + 1).max().unwrap();
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

                eprintln!(
                    "Building... {:#?}",
                    configured_bazel_runner.bazel_command_line.remaining_args
                );
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

                    eprintln!(
                        "Testing... {:#?}",
                        configured_bazel_runner.bazel_command_line.remaining_args
                    );

                    let result = configured_bazel_runner.run_command_line().await?;
                    if result.final_exit_code != 0 {
                        continue;
                    }
                }

                eprintln!(
                    "Operating at distance {}, all targets built and tested that were eligble.",
                    cur_distance
                );
            }
            if cur_distance >= max_distance {
                cur_distance = 1;
                dirty_files.clear();
            } else {
                cur_distance += 1;
            }
        }
    }

    Ok(false)
}

#[cfg(test)]
mod tests {}
