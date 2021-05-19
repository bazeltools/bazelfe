use crate::config::CommandLineRewriter;
use crate::{
    bazel_command_line_parser::{BuiltInAction, ParsedCommandLine},
    config::command_line_rewriter::TestActionMode,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RewriteCommandLineError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(super::UserReportError),
}

pub async fn rewrite_command_line(
    bazel_command_line: &mut ParsedCommandLine,
    command_line_rewriter: &CommandLineRewriter,
    daemon_client: &Option<crate::bazel_runner_daemon::daemon_service::RunnerDaemonClient>,
) -> Result<(), RewriteCommandLineError> {
    if bazel_command_line.action
        == Some(crate::bazel_command_line_parser::Action::BuiltIn(
            BuiltInAction::Test,
        ))
    {
        if bazel_command_line.remaining_args.is_empty() {
            match &command_line_rewriter.test {
                TestActionMode::EmptyTestToLocalRepo(cfg) => {
                    bazel_command_line
                        .remaining_args
                        .push(cfg.command_to_use.clone());
                }
                TestActionMode::EmptyTestToFail => {
                    Err(RewriteCommandLineError::UserErrorReport(super::UserReportError("No test target specified.\nUnlike other build tools, bazel requires you specify which test target to test.\nTo test the whole repo add //... to the end. But beware this could be slow!".to_owned())))?;
                }
                TestActionMode::Passthrough => {}
                TestActionMode::SuggestTestTarget(cfg) => {
                    if let Some(daemon_cli) = daemon_client.as_ref() {
                        let mut invalidated_targets = vec![];

                        for distance in 0..(cfg.distance_to_expand + 1) {
                            let recently_invalidated_targets = daemon_cli
                                .recently_invalidated_targets(tarpc::context::current(), distance)
                                .await;
                            invalidated_targets.extend(
                                recently_invalidated_targets
                                    .into_iter()
                                    .map(|e| (distance, e)),
                            );
                        }
                        if !invalidated_targets.is_empty() {
                            use trim_margin::MarginTrimmable;

                            invalidated_targets.sort_by_key(|e| e.0);
                            use std::collections::HashSet;
                            let mut seen_targets: HashSet<String> = HashSet::default();
                            let mut buf = String::from("");
                            invalidated_targets.into_iter().for_each(|(_, targets)| {
                                targets.iter().for_each(|target| {
                                    if target.is_test() {
                                        if !seen_targets.contains(target.target_label()) {
                                            seen_targets.insert(target.target_label().clone());
                                            buf.push_str(&format!("\n|{}", target.target_label()));
                                        }
                                    }
                                });
                            });

                            Err(RewriteCommandLineError::UserErrorReport(
                                super::UserReportError(
                                    format!(
                                        r#"|No test target specified.
                                    | Suggestions: 
                                    |{}
                                    |"#,
                                        buf
                                    )
                                    .trim_margin()
                                    .unwrap()
                                    .to_owned(),
                                ),
                            ))?;
                        }
                    } else {
                        Err(RewriteCommandLineError::UserErrorReport(super::UserReportError(
                                "Configured to suggest possible test targets to run, but no daemon is running".to_owned())))?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::bazel_command_line_parser::*;
    use crate::config::command_line_rewriter::*;
    use std::path::PathBuf;
    #[tokio::test]
    async fn test_passthrough_args() {
        let mut passthrough_command_line = ParsedCommandLine {
            bazel_binary: PathBuf::from("bazel"),
            startup_options: Vec::default(),
            action: Some(Action::BuiltIn(BuiltInAction::Test)),
            action_options: Vec::default(),
            remaining_args: vec!["bar".to_string()],
        };

        let _ = rewrite_command_line(
            &mut passthrough_command_line,
            &CommandLineRewriter::default(),
            &None,
        )
        .await
        .unwrap();

        assert_eq!(
            passthrough_command_line,
            ParsedCommandLine {
                bazel_binary: PathBuf::from("bazel"),
                startup_options: Vec::default(),
                action: Some(Action::BuiltIn(BuiltInAction::Test)),
                action_options: Vec::default(),
                remaining_args: vec!["bar".to_string()],
            }
        );
    }

    #[tokio::test]
    async fn test_rewrite_empty_test() {
        let mut passthrough_command_line = ParsedCommandLine {
            bazel_binary: PathBuf::from("bazel"),
            startup_options: Vec::default(),
            action: Some(Action::BuiltIn(BuiltInAction::Test)),
            action_options: Vec::default(),
            remaining_args: vec![],
        };
        let rewrite_config = CommandLineRewriter {
            test: TestActionMode::EmptyTestToLocalRepo(EmptyTestToLocalRepoCfg::default()),
        };
        let _ = rewrite_command_line(&mut passthrough_command_line, &rewrite_config, &None)
            .await
            .unwrap();

        assert_eq!(
            passthrough_command_line,
            ParsedCommandLine {
                bazel_binary: PathBuf::from("bazel"),
                startup_options: Vec::default(),
                action: Some(Action::BuiltIn(BuiltInAction::Test)),
                action_options: Vec::default(),
                remaining_args: vec!["//...".to_string()],
            }
        );
    }

    #[tokio::test]
    async fn test_rewrite_empty_test_to_fail() {
        let mut passthrough_command_line = ParsedCommandLine {
            bazel_binary: PathBuf::from("bazel"),
            startup_options: Vec::default(),
            action: Some(Action::BuiltIn(BuiltInAction::Test)),
            action_options: Vec::default(),
            remaining_args: vec![],
        };

        let rewrite_config = CommandLineRewriter {
            test: TestActionMode::EmptyTestToFail,
        };
        let ret = rewrite_command_line(&mut passthrough_command_line, &rewrite_config, &None).await;

        assert_eq!(true, ret.is_err());

        match ret {
            Ok(_) => panic!("Expected to get an error condition from the call"),
            Err(e) => match e {
                RewriteCommandLineError::UserErrorReport(err) => {
                    assert_eq!(true, err.0.contains("No test target specified"));
                }
            },
        }
    }
}
