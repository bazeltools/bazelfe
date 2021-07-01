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
                TestActionMode::SuggestTestTarget(_cfg) => {
                    Err(RewriteCommandLineError::UserErrorReport(super::UserReportError(
                                "Configured to suggest possible test targets to run, but no daemon is running".to_owned())))?;
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
        let _ = rewrite_command_line(&mut passthrough_command_line, &rewrite_config)
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
        let ret = rewrite_command_line(&mut passthrough_command_line, &rewrite_config).await;

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
