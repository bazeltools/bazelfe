use crate::config::command_line_rewriter::TestActionMode;
use crate::config::CommandLineRewriter;
use crate::jvm_indexer::bazel_query::BazelQuery;

use bazelfe_bazel_wrapper::bazel_command_line_parser::{
    self, parse_bazel_command_line, Action, CommandLineParsingError,
};
use bazelfe_bazel_wrapper::bazel_subprocess_wrapper::UserReportError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomAction {
    AutoTest,
    TestFile,
    BuildFile,
}
impl CustomAction {
    pub fn to_string(&self) -> String {
        match self {
            CustomAction::AutoTest => "autotest",
            CustomAction::TestFile => "test_file",
            CustomAction::BuildFile => "build_file",
        }
        .to_string()
    }
}

pub fn parse_commandline_with_custom_command_line_options(
    command_line: &[String],
) -> Result<ParsedCommandLine, CommandLineParsingError> {
    let mut m: HashMap<String, BuiltInAction> = Default::default();
    m.insert("autotest".to_string(), BuiltInAction::Test);
    m.insert("test_file".to_string(), BuiltInAction::Test);
    m.insert("build_file".to_string(), BuiltInAction::Build);
    parse_bazel_command_line(command_line, m)
}

pub fn parse_custom_action(input: &str) -> Result<CustomAction, RewriteCommandLineError> {
    match input {
        "autotest" => Ok(CustomAction::AutoTest),
        "test_file" => Ok(CustomAction::TestFile),
        "build_file" => Ok(CustomAction::BuildFile),
        _ => Err(RewriteCommandLineError::UserErrorReport(UserReportError(
            format!("Unknown custom command passed in {}", input),
        ))),
    }
}

use std::collections::HashMap;
#[cfg(feature = "bazelfe-daemon")]
use std::fmt::Write;

use bazelfe_bazel_wrapper::bazel_command_line_parser::{BuiltInAction, ParsedCommandLine};
#[cfg(feature = "bazelfe-daemon")]
use bazelfe_protos::bazel_tools::daemon_service::daemon_service_client::DaemonServiceClient;
use thiserror::Error;
#[cfg(feature = "bazelfe-daemon")]
use tonic::transport::Channel;

use super::test_file_to_target;

#[derive(Error, Debug)]
pub enum RewriteCommandLineError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(UserReportError),
}

pub async fn rewrite_command_line<B: BazelQuery>(
    bazel_command_line: &mut ParsedCommandLine,
    command_line_rewriter: &CommandLineRewriter,
    #[cfg(feature = "bazelfe-daemon")] daemon_client: &mut Option<DaemonServiceClient<Channel>>,
    bazel_query: B,
) -> Result<(), RewriteCommandLineError> {
    if let Some(Action::Custom(cust)) = bazel_command_line.action.as_ref() {
        let custom_action = parse_custom_action(cust)?;
        match custom_action {
            CustomAction::AutoTest => todo!(),
            CustomAction::TestFile => {
                test_file_to_target::run(bazel_command_line, BuiltInAction::Test, bazel_query)
                    .await;
            }
            CustomAction::BuildFile => {
                return test_file_to_target::run(
                    bazel_command_line,
                    BuiltInAction::Build,
                    bazel_query,
                )
                .await;
            }
        }
    }

    if bazel_command_line.action
        == Some(bazel_command_line_parser::Action::BuiltIn(
            BuiltInAction::Test,
        ))
    {
        let mut test_action_modes = vec![&command_line_rewriter.test];
        while let Some(test_action_mode) = test_action_modes.pop() {
            match test_action_mode {
                TestActionMode::EmptyTestToLocalRepo(cfg)
                    if bazel_command_line.remaining_args.is_empty() =>
                {
                    bazel_command_line
                        .remaining_args
                        .push(cfg.command_to_use.clone());
                }
                TestActionMode::EmptyTestToFail if bazel_command_line.remaining_args.is_empty() => {
                    return Err(RewriteCommandLineError::UserErrorReport(UserReportError("No test target specified.\nUnlike other build tools, bazel requires you specify which test target to test.\nTo test the whole repo add //... to the end. But beware this could be slow!".to_owned())));
                }
                TestActionMode::Passthrough => {}

                #[allow(unused)]
                TestActionMode::SuggestTestTarget(cfg)
                    if bazel_command_line.remaining_args.is_empty() =>
                {
                    #[cfg(feature = "bazelfe-daemon")]
                    if let Some(daemon_cli) = daemon_client.as_mut() {
                        let mut invalidated_targets = vec![];

                        for distance in 0..(cfg.distance_to_expand + 1) {
                            let recently_invalidated_targets = daemon_cli
                                .recently_invalidated_targets(bazelfe_protos::bazel_tools::daemon_service::RecentlyInvalidatedTargetsRequest {
                                    distance,
                                })
                                .await;
                            invalidated_targets.extend(
                                recently_invalidated_targets
                                    .into_iter()
                                    .map(|e| e.into_inner().targets.unwrap_or_default())
                                    .map(|e| (distance, e)),
                            );
                        }
                        if !invalidated_targets.is_empty() {
                            use trim_margin::MarginTrimmable;

                            invalidated_targets.sort_by_key(|e| e.0);
                            use std::collections::HashSet;
                            let mut seen_targets: HashSet<String> = HashSet::default();
                            let mut buf = String::from("");
                            invalidated_targets.into_iter().for_each(|(_, mut targets)| {
                            targets.targets.iter_mut().for_each(|target| {
                                match target.target_response.take().unwrap() {
                                    bazelfe_protos::bazel_tools::daemon_service::target::TargetResponse::BuildLabel(_) => (),
                                    bazelfe_protos::bazel_tools::daemon_service::target::TargetResponse::TestLabel(label) => {
                                        if(!seen_targets.contains(&label))
                                        {
                                            seen_targets.insert(label.clone());
                                            write!(buf, "\n|{}", label);
                                        }
                                    },
                                }
                            });
                        });

                            let suggestion_str = if buf.is_empty() {
                                "Daemon hasn't noticed any changes to suggest test targets"
                                    .to_string()
                            } else {
                                format!(
                                    r#"Suggestions:
                            |{}
                            |"#,
                                    buf
                                )
                            };
                            return Err(RewriteCommandLineError::UserErrorReport(UserReportError(
                                format!(
                                    r#"|No test target specified.
                                |{}"#,
                                    suggestion_str
                                )
                                .trim_margin()
                                .unwrap(),
                            )));
                        }
                    } else {
                        return Err(RewriteCommandLineError::UserErrorReport(UserReportError(
                            "Configured to suggest possible test targets to run, but no daemon is running".to_owned())));
                    }

                    #[cfg(not(feature = "bazelfe-daemon"))]
                Err(RewriteCommandLineError::UserErrorReport(UserReportError(
                    "Bazelfe is configured to suggest possible test targets to run, however the daemon is not included in this build".to_owned())))?;
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::config::command_line_rewriter::*;
    use crate::jvm_indexer::bazel_query::ExecuteResult;
    use bazel_command_line_parser::*;
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

        rewrite_command_line(
            &mut passthrough_command_line,
            &CommandLineRewriter::default(),
            #[cfg(feature = "bazelfe-daemon")]
            &mut None,
            TestBazelQuery::success("good".into()),
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
        rewrite_command_line(
            &mut passthrough_command_line,
            &rewrite_config,
            #[cfg(feature = "bazelfe-daemon")]
            &mut None,
            TestBazelQuery::success("good".into()),
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
                remaining_args: vec!["//...".to_string()],
            }
        );
    }

    #[derive(Debug)]
    struct TestBazelQuery(ExecuteResult);
    impl TestBazelQuery {
        pub fn success(str: String) -> TestBazelQuery {
            TestBazelQuery(ExecuteResult {
                exit_code: 0,
                stdout: str,
                stdout_raw: Default::default(),
                stderr: "".into(),
                stderr_raw: Default::default(),
            })
        }
    }

    #[async_trait::async_trait]
    impl BazelQuery for TestBazelQuery {
        async fn execute(&self, _args: &[String]) -> ExecuteResult {
            self.0.clone()
        }
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
        let ret = rewrite_command_line(
            &mut passthrough_command_line,
            &rewrite_config,
            #[cfg(feature = "bazelfe-daemon")]
            &mut None,
            TestBazelQuery::success("good".into()),
        )
        .await;

        assert!(ret.is_err());

        match ret {
            Ok(_) => panic!("Expected to get an error condition from the call"),
            Err(RewriteCommandLineError::UserErrorReport(err)) => {
                assert!(err.0.contains("No test target specified"));
            }
        }
    }

    #[tokio::test]
    async fn test_test_file_rewrite() {
        let temp_dir = tempfile::tempdir_in(".").expect("should be able to make a tempdir");
        let build_path = temp_dir.path().join("BUILD");
        let scala_path = temp_dir.path().join("foo.scala");

        std::fs::File::create(&build_path).expect("should have created BUILD");
        std::fs::File::create(&scala_path).expect("should have created foo.scala");

        let mut passthrough_command_line = ParsedCommandLine {
            bazel_binary: PathBuf::from("bazel"),
            startup_options: Vec::default(),
            action: Some(Action::Custom(CustomAction::TestFile.to_string())),
            action_options: Vec::default(),
            remaining_args: vec![
                temp_dir
                    .path()
                    .file_name()
                    .expect("tmp dir isn't empty")
                    .to_string_lossy()
                    .to_string()
                    + "/foo.scala",
            ],
        };

        let rewrite_config = Default::default();
        let ret = rewrite_command_line(
            &mut passthrough_command_line,
            &rewrite_config,
            #[cfg(feature = "bazelfe-daemon")]
            &mut None,
            TestBazelQuery::success("foo\nbar".into()),
        )
        .await;

        assert!(ret.is_ok(), "{:#?}", ret.err());
        assert_eq!(
            passthrough_command_line.action,
            Some(Action::BuiltIn(BuiltInAction::Test))
        )
    }

    #[tokio::test]
    async fn test_build_file_rewrite() {
        let temp_dir = tempfile::tempdir_in(".").expect("should be able to make a tempdir");
        let build_path = temp_dir.path().join("BUILD");
        let scala_path = temp_dir.path().join("foo.scala");

        std::fs::File::create(&build_path).expect("should have created BUILD");
        std::fs::File::create(&scala_path).expect("should have created foo.scala");

        let mut passthrough_command_line = ParsedCommandLine {
            bazel_binary: PathBuf::from("bazel"),
            startup_options: Vec::default(),
            action: Some(Action::Custom(CustomAction::BuildFile.to_string())),
            action_options: Vec::default(),
            remaining_args: vec![
                temp_dir
                    .path()
                    .file_name()
                    .expect("tmp dir isn't empty")
                    .to_string_lossy()
                    .to_string()
                    + "/foo.scala",
            ],
        };

        let rewrite_config = Default::default();
        let ret = rewrite_command_line(
            &mut passthrough_command_line,
            &rewrite_config,
            #[cfg(feature = "bazelfe-daemon")]
            &mut None,
            TestBazelQuery::success("foo\nbar".into()),
        )
        .await;

        assert!(ret.is_ok(), "{:#?}", ret.err());
        assert_eq!(
            passthrough_command_line.action,
            Some(Action::BuiltIn(BuiltInAction::Build))
        )
    }
}
