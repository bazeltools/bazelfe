use bazelfe_bazel_wrapper::bazel_command_line_parser::{self, ParsedCommandLine};
use bazelfe_bazel_wrapper::bazel_subprocess_wrapper::{
    BazelWrapperBuilder, BazelWrapperError, UserReportError,
};
use std::env;
use tokio::sync::Mutex;

use crate::bazel_query::{BazelQueryEngine, RealBazelQueryEngine};
use crate::bazel_runner::command_line_rewriter_action::{parse_custom_action, CustomAction};

use crate::bazel_runner::configured_bazel_runner::ConfiguredBazelRunner;
use crate::buildozer_driver;
use crate::config::Config;
use crate::hydrated_stream_processors::index_new_results::IndexNewResults;
use crate::hydrated_stream_processors::process_bazel_failures::ProcessBazelFailures;

use std::sync::Arc;

use thiserror::Error;

use super::command_line_rewriter_action;

#[derive(Error, Debug)]
pub enum BazelRunnerError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(UserReportError),
    #[error(transparent)]
    CommandLineRewriterActionError(command_line_rewriter_action::RewriteCommandLineError),

    #[error("Unclassified or otherwise unknown error occured: `{0:?}`")]
    Unknown(Box<dyn std::error::Error>),
}

impl From<command_line_rewriter_action::RewriteCommandLineError> for BazelRunnerError {
    fn from(inner: command_line_rewriter_action::RewriteCommandLineError) -> Self {
        match inner {
            command_line_rewriter_action::RewriteCommandLineError::UserErrorReport(ex) => {
                BazelRunnerError::UserErrorReport(ex)
            }
        }
    }
}

impl From<Box<dyn std::error::Error>> for BazelRunnerError {
    fn from(inner: Box<dyn std::error::Error>) -> Self {
        BazelRunnerError::Unknown(inner)
    }
}

impl From<BazelWrapperError> for BazelRunnerError {
    fn from(inner: BazelWrapperError) -> Self {
        match inner {
            BazelWrapperError::Unknown(o) => Self::Unknown(o),
            BazelWrapperError::UserErrorReport(u) => Self::UserErrorReport(u),
        }
    }
}
pub struct BazelRunner {
    pub config: Config,
    pub bazel_command_line: ParsedCommandLine,
}

impl BazelRunner {
    pub async fn run(mut self) -> Result<i32, BazelRunnerError> {
        let _rng = rand::thread_rng();

        bazelfe_bazel_wrapper::bazel_subprocess_wrapper::register_ctrlc_handler();

        debug!("Based on custom action if present, overriding the daemon option");
        if let Some(bazel_command_line_parser::Action::Custom(cust_str)) =
            self.bazel_command_line.action.as_ref()
        {
            if CustomAction::AutoTest == parse_custom_action(cust_str)? {
                self.config.daemon_config.enabled = true;
            }
        }

        let config = Arc::new(self.config);

        debug!("Loading index..");
        let index_table = match &config.index_input_location {
            Some(p) => {
                if p.exists() {
                    let mut src_f = std::fs::File::open(p).unwrap();
                    crate::index_table::IndexTable::read(&mut src_f).unwrap_or_default()
                } else {
                    crate::index_table::IndexTable::new()
                }
            }
            None => crate::index_table::IndexTable::new(),
        };

        debug!("Index loading complete..");

        let bazel_query: Arc<Mutex<Box<dyn crate::jvm_indexer::bazel_query::BazelQuery>>> =
            Arc::new(Mutex::new(Box::new(
                crate::jvm_indexer::bazel_query::from_binary_path(
                    &self.bazel_command_line.bazel_binary.clone(),
                ),
            )));

        let bazel_query_engine: Arc<dyn BazelQueryEngine> =
            Arc::new(RealBazelQueryEngine::new(bazel_query));
        let process_build_failures = Arc::new(ProcessBazelFailures::new(
            index_table.clone(),
            buildozer_driver::from_binary_path(
                config
                    .buildozer_path
                    .as_ref()
                    .expect("Unable to find a config for buildozer, error."),
            ),
            crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunnerImpl(),
            Arc::clone(&config),
            Arc::clone(&bazel_query_engine),
        )?);

        let addr: Option<std::net::SocketAddr> = config
            .bes_server_bind_address
            .map(|s| s.to_owned())
            .or_else(|| {
                env::var("BIND_ADDRESS")
                    .ok()
                    .map(|e| e.parse().expect("can't parse BIND_ADDRESS variable"))
            });

        let bazel_wrapper_builder = BazelWrapperBuilder {
            bes_server_bind_address: addr,
            processors: vec![
                process_build_failures.clone(),
                Arc::new(IndexNewResults::new(
                    index_table.clone(),
                    &config.indexer_config,
                )),
            ],
        };

        let bazel_wrapper = bazel_wrapper_builder.build().await?;

        #[cfg(feature = "bazelfe-daemon")]
        let runner_daemon = if let Some(bazel_command_line_parser::Action::BuiltIn(
            bazel_command_line_parser::BuiltInAction::Shutdown,
        )) = self.bazel_command_line.action
        {
            crate::bazel_runner_daemon::daemon_manager::try_kill_server_from_cfg(
                &config.daemon_config,
            )
            .await;
            None
        } else {
            crate::bazel_runner_daemon::daemon_manager::connect_to_server(
                &config.daemon_config,
                &self.bazel_command_line.bazel_binary.clone(),
            )
            .await?
        };

        let configured_bazel_runner = ConfiguredBazelRunner::new(
            Arc::clone(&config),
            bazel_wrapper,
            #[cfg(feature = "bazelfe-daemon")]
            runner_daemon,
            index_table.clone(),
            self.bazel_command_line.clone(),
            process_build_failures,
        );

        let final_exit_code_res = configured_bazel_runner.run().await;

        if index_table.is_mutated() {
            debug!("Writing out index file...");

            if let Some(target_path) = &config.index_input_location {
                if let Some(parent) = target_path.parent() {
                    std::fs::create_dir_all(parent).unwrap();
                }
                let mut temp_path = target_path.clone();
                temp_path.set_extension("tmp");

                let mut file = std::fs::File::create(&temp_path).unwrap();
                index_table.write(&mut file).await;
                drop(file);
                std::fs::rename(temp_path, target_path)
                    .expect("Expected to be able to rename our temp path into the final location.");
            }
            debug!("Index write complete.");
        }
        Ok(final_exit_code_res?)
    }
}
