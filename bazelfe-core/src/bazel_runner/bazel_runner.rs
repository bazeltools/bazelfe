use bazel_runner::configured_bazel_runner::ConfiguredBazelRunner;
use std::env;
use tonic::transport::Server;

use bazelfe_protos::*;

use crate::{bazel_command_line_parser::ParsedCommandLine, buildozer_driver};

use crate::config::Config;
use crate::{
    bazel_runner,
    hydrated_stream_processors::{
        event_stream_listener::EventStreamListener, index_new_results::IndexNewResults,
        process_bazel_failures::ProcessBazelFailures, BazelEventHandler,
    },
};
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::sync::Arc;

use thiserror::Error;

use super::command_line_rewriter_action;

#[derive(Error, Debug)]
pub enum BazelRunnerError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(super::UserReportError),
    #[error(transparent)]
    CommandLineRewriterActionError(command_line_rewriter_action::RewriteCommandLineError),

    #[error("Unclassified or otherwise unknown error occured: `{0}`")]
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

pub struct BazelRunner {
    pub config: Config,
    pub bazel_command_line: ParsedCommandLine,
}

impl BazelRunner {
    pub async fn run(mut self) -> Result<i32, BazelRunnerError> {
        let mut rng = rand::thread_rng();

        bazel_runner::register_ctrlc_handler();

        debug!("Based on custom action if present, overriding the daemon option");
        if let Some(action) = self.bazel_command_line.action.as_ref() {
            if let crate::bazel_command_line_parser::Action::Custom(
                crate::bazel_command_line_parser::CustomAction::AutoTest,
            ) = action
            {
                self.config.daemon_config.enabled = true;
            }
        }

        let config = Arc::new(self.config);

        debug!("Loading index..");
        let index_table = match &config.index_input_location {
            Some(p) => {
                if p.exists() {
                    let mut src_f = std::fs::File::open(p).unwrap();
                    crate::index_table::IndexTable::read(&mut src_f)
                } else {
                    crate::index_table::IndexTable::new()
                }
            }
            None => crate::index_table::IndexTable::new(),
        };

        debug!("Index loading complete..");

        let process_build_failures = Arc::new(ProcessBazelFailures::new(
            index_table.clone(),
            buildozer_driver::from_binary_path(
                &config
                    .buildozer_path
                    .as_ref()
                    .expect("Unable to find a config for buildozer, error."),
            ),
            crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunnerImpl(),
            Arc::clone(&config),
        )?);
        let processors: Vec<Arc<dyn BazelEventHandler>> = vec![
            process_build_failures.clone(),
            Arc::new(IndexNewResults::new(index_table.clone())),
        ];
        let aes = EventStreamListener::new(processors);

        let default_port = {
            let rand_v: u16 = rng.gen();
            40000 + (rand_v % 3000)
        };

        let addr: std::net::SocketAddr = config
            .bes_server_bind_address
            .map(|s| s.to_owned())
            .unwrap_or_else(|| {
                env::var("BIND_ADDRESS")
                    .ok()
                    .unwrap_or_else(|| format!("127.0.0.1:{}", default_port).into())
                    .parse()
                    .expect("can't parse BIND_ADDRESS variable")
            });

        debug!("Services listening on {}", addr);

        let (bes, sender_arc, _) =
            crate::build_events::build_event_server::build_bazel_build_events_service();

        let bes_port: u16 = addr.port();

        let _service_fut = tokio::spawn(async move {
            Server::builder()
                .add_service(PublishBuildEventServer::new(bes))
                .serve(addr)
                .await
                .unwrap();
        });

        let runner_daemon = crate::bazel_runner_daemon::daemon_manager::connect_to_server(
            &config.daemon_config,
            &self.bazel_command_line.bazel_binary.clone(),
        )
        .await?;

        let configured_bazel =
            super::configured_bazel_runner::ConfiguredBazel::new(&sender_arc, aes, bes_port);

        let configured_bazel_runner = ConfiguredBazelRunner::new(
            Arc::clone(&config),
            configured_bazel,
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
