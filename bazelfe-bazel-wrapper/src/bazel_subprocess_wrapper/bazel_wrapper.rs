use log::debug;
use std::env;
use tokio::sync::Mutex;
use tonic::transport::Server;

use bazelfe_protos::*;

use crate::bazel_command_line_parser::ParsedCommandLine;
use crate::bep::build_events;
use crate::bep::BazelEventHandler;
use crate::bep::EventStreamListener;

use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::sync::Arc;
use thiserror::Error;

use super::ExecuteResult;
use std::collections::HashMap;

use crate::bep::build_events::build_event_server::BuildEventAction;
use crate::bep::build_events::hydrated_stream::HydratedInfo;

use crate::bep::build_events::build_event_server::bazel_event;

use tokio::sync::RwLock;

pub struct BazelWrapper<T> {
    sender_arc:
        Arc<Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>>,
    pub aes: EventStreamListener<T>,
    bes_port: u16,
}

impl<T> BazelWrapper<T>
where
    T: Send + 'static,
{
    pub(super) fn new(
        sender_arc: &Arc<
            Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
        >,
        aes: EventStreamListener<T>,
        bes_port: u16,
    ) -> Self {
        Self {
            sender_arc: sender_arc.clone(),
            aes,
            bes_port,
        }
    }

    pub async fn spawn_bazel_attempt(
        &self,
        bazel_command_line: &ParsedCommandLine,
        pipe_output: bool,
        user_stream_handler: async_channel::Sender<T>,
    ) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
        let (tx, rx) = async_channel::unbounded();
        {
            let mut locked = self.sender_arc.lock().await;
            *locked = Some(tx);
        };
        let error_stream = HydratedInfo::build_transformer(rx);

        let target_extracted_stream = self.aes.handle_stream(error_stream);

        let recv_task = tokio::spawn(async move {
            while let Ok(action) = target_extracted_stream.recv().await {
                user_stream_handler.send(action).await;
            }
        });

        let res =
            super::execute_bazel_output_control(bazel_command_line, self.bes_port, pipe_output)
                .await?;
        {
            let mut locked = self.sender_arc.lock().await;
            locked.take();
        };
        recv_task.await.unwrap();
        Ok(res)
    }
}
