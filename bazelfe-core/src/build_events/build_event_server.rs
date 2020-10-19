use tonic::{Request, Response, Status};

use bazelfe_protos::*;
use futures::{Stream, StreamExt};

use google::devtools::build::v1::publish_build_event_server::PublishBuildEvent;
use google::devtools::build::v1::{
    PublishBuildToolEventStreamRequest, PublishBuildToolEventStreamResponse,
    PublishLifecycleEventRequest,
};
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

pub mod bazel_event {
    use super::*;
    use ::prost::Message;

    #[derive(Clone, PartialEq, Debug)]
    pub struct BazelBuildEvent {
        pub event: Evt,
    }
    impl BazelBuildEvent {
        pub fn transform_from(
            inbound_evt: &mut PublishBuildToolEventStreamRequest,
        ) -> Option<BazelBuildEvent> {
            let mut inner_data = inbound_evt
                .ordered_build_event
                .take()
                .as_mut()
                .and_then(|inner| inner.event.take());
            let _event_time = inner_data.as_mut().and_then(|e| e.event_time.take());
            let _event = inner_data.and_then(|mut e| e.event.take());

            let decoded_evt = match _event {
                Some(inner) => match inner {
                    google::devtools::build::v1::build_event::Event::BazelEvent(e) => {
                        let v = build_event_stream::BuildEvent::decode(&*e.value).unwrap();

                        let target_configured_evt: Option<TargetConfiguredEvt> = {
                            let target_kind_opt = v.payload.as_ref().and_then(|e| match e {
                                build_event_stream::build_event::Payload::Configured(cfg) => {
                                    Some(cfg.target_kind.replace(" rule", ""))
                                }
                                _ => None,
                            });
                            let target_label_opt = v
                                .id
                                .as_ref()
                                .and_then(|e| e.id.as_ref())
                                .and_then(|e| match e {
                                    build_event_stream::build_event_id::Id::TargetConfigured(
                                        target_configured_id,
                                    ) => Some(target_configured_id.label.clone()),
                                    _ => None,
                                });

                            target_kind_opt.and_then(|e| {
                                target_label_opt.map(|u| TargetConfiguredEvt {
                                    rule_kind: e,
                                    label: u,
                                })
                            })
                        };

                        let aborted: Option<Evt> = {
                            let abort_info = v.payload.as_ref().and_then(|e| match e {
                                build_event_stream::build_event::Payload::Aborted(cfg) => Some((
                                    build_event_stream::aborted::AbortReason::from_i32(cfg.reason),
                                    cfg.description.clone(),
                                )),
                                _ => None,
                            });
                            let target_label_opt =
                                v.id.as_ref()
                                    .and_then(|e| e.id.as_ref())
                                    .and_then(|e| match e {
                                        build_event_stream::build_event_id::Id::ConfiguredLabel(
                                            configured_label_id,
                                        ) => Some(configured_label_id.label.clone()),
                                        _ => None,
                                    });

                            abort_info.map(|(reason, description)| {
                                Evt::Aborted(AbortedEvt {
                                    label: target_label_opt,
                                    reason: reason,
                                    description: description,
                                })
                            })
                        };

                        let progress_info: Option<Evt> = v.payload.as_ref().and_then(|e| match e {
                            build_event_stream::build_event::Payload::Progress(cfg) => {
                                if cfg.stdout.is_empty() && cfg.stderr.is_empty() {
                                    None
                                } else {
                                    Some(Evt::Progress(ProgressEvt {
                                        stdout: cfg.stdout.clone(),
                                        stderr: cfg.stderr.clone(),
                                    }))
                                }
                            }
                            _ => None,
                        });

                        let action_info: Option<Evt> = {
                            let target_label_opt =
                                v.id.as_ref()
                                    .and_then(|e| e.id.as_ref())
                                    .and_then(|e| match e {
                                        build_event_stream::build_event_id::Id::ActionCompleted(
                                            action_completed_id,
                                        ) => Some(action_completed_id.label.clone()),
                                        _ => None,
                                    });

                            target_label_opt.and_then(|label| {
                                v.payload.as_ref().and_then(|e| match e {
                                    build_event_stream::build_event::Payload::Action(
                                        action_executed,
                                    ) => {
                                        let stdout = action_executed
                                            .stdout
                                            .as_ref()
                                            .and_then(|e| e.file.clone());
                                        let stderr = action_executed
                                            .stderr
                                            .as_ref()
                                            .and_then(|e| e.file.clone());

                                        Some(Evt::ActionCompleted(ActionCompletedEvt {
                                            success: action_executed.success,
                                            label: label,
                                            stdout: stdout,
                                            stderr: stderr,
                                        }))
                                    }
                                    _ => None,
                                })
                            })
                        };

                        let named_set_of_files: Option<Evt> = {
                            let fileset_id =
                                v.id.as_ref()
                                    .and_then(|e| e.id.as_ref())
                                    .and_then(|e| match e {
                                        build_event_stream::build_event_id::Id::NamedSet(
                                            fileset_id,
                                        ) => Some(fileset_id.id.clone()),
                                        _ => None,
                                    });

                            fileset_id.and_then(|id| {
                                v.payload.as_ref().and_then(|e| match e {
                                    build_event_stream::build_event::Payload::NamedSetOfFiles(
                                        named_set_of_files,
                                    ) => Some(Evt::NamedSetOfFiles {
                                        id: id,
                                        named_set_of_files: named_set_of_files.clone(),
                                    }),
                                    _ => None,
                                })
                            })
                        };

                        let target_complete: Option<Evt> = {
                            let target_label_opt =
                                v.id.as_ref()
                                    .and_then(|e| e.id.as_ref())
                                    .and_then(|e| match e {
                                        build_event_stream::build_event_id::Id::TargetCompleted(
                                            target_completed_id,
                                        ) => Some(target_completed_id.label.clone()),
                                        _ => None,
                                    });

                            target_label_opt.and_then(|label| {
                                v.payload.as_ref().and_then(|e| match e {
                                    build_event_stream::build_event::Payload::Completed(
                                        target_completed,
                                    ) => Some(Evt::TargetCompleted(TargetCompletedEvt {
                                        success: target_completed.success,
                                        label: label,
                                        output_groups: target_completed.output_group.clone(),
                                    })),
                                    _ => None,
                                })
                            })
                        };

                        let test_outputs: Option<Evt> = {
                            let failed_file_data: Option<Vec<build_event_stream::file::File>> =
                                v.payload.as_ref().and_then(|e| match e {
                                    build_event_stream::build_event::Payload::TestSummary(cfg) => {
                                        Some(
                                            cfg.failed
                                                .iter()
                                                .flat_map(|e| e.file.clone().into_iter())
                                                .collect(),
                                        )
                                    }
                                    _ => None,
                                });

                            let target_label_opt =
                                v.id.as_ref()
                                    .and_then(|e| e.id.as_ref())
                                    .and_then(|e| match e {
                                        build_event_stream::build_event_id::Id::TestSummary(
                                            test_summary_id,
                                        ) => Some(test_summary_id.label.clone()),
                                        _ => None,
                                    });

                            failed_file_data.and_then(|failed_files| {
                                target_label_opt.map(|u| {
                                    Evt::TestFailure(TestFailureEvt {
                                        label: u,
                                        failed_files: failed_files,
                                    })
                                })
                            })
                        };

                        if let Some(e) = target_configured_evt {
                            Evt::TargetConfigured(e)
                        } else if let Some(e) = action_info {
                            e
                        } else if let Some(e) = target_complete {
                            e
                        } else if let Some(e) = test_outputs {
                            e
                        } else if let Some(e) = named_set_of_files {
                            e
                        } else if let Some(e) = aborted {
                            e
                        } else if let Some(e) = progress_info {
                            e
                        } else {
                            Evt::BazelEvent(v)
                        }
                    }
                    other => Evt::UnknownEvent(format!("{:?}", other)),
                },
                None => Evt::UnknownEvent("Missing Event".to_string()),
            };

            info!("Decoded evt: {:?}", decoded_evt);
            Some(BazelBuildEvent { event: decoded_evt })
        }
    }
    #[derive(Clone, PartialEq, Debug)]
    pub struct ActionCompletedEvt {
        pub success: bool,
        pub label: String,
        pub stdout: Option<build_event_stream::file::File>,
        pub stderr: Option<build_event_stream::file::File>,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct AbortedEvt {
        pub label: Option<String>,
        pub reason: Option<build_event_stream::aborted::AbortReason>,
        pub description: String,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct ProgressEvt {
        pub stdout: String,
        pub stderr: String,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct TestFailureEvt {
        pub label: String,
        pub failed_files: Vec<build_event_stream::file::File>,
    }
    #[derive(Clone, PartialEq, Debug)]
    pub struct TargetConfiguredEvt {
        pub label: String,
        pub rule_kind: String,
    }

    #[derive(Clone, PartialEq, Debug)]
    pub struct TargetCompletedEvt {
        pub label: String,
        pub success: bool,
        pub output_groups: Vec<build_event_stream::OutputGroup>,
    }
    #[derive(Clone, PartialEq, Debug)]
    pub enum Evt {
        BazelEvent(build_event_stream::BuildEvent),
        TargetConfigured(TargetConfiguredEvt),
        ActionCompleted(ActionCompletedEvt),
        TestFailure(TestFailureEvt),
        Progress(ProgressEvt),
        Aborted(AbortedEvt),
        TargetCompleted(TargetCompletedEvt),
        NamedSetOfFiles {
            id: String,
            named_set_of_files: build_event_stream::NamedSetOfFiles,
        },
        UnknownEvent(String),
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum BuildEventAction<T> {
    BuildEvent(T),
    LifecycleEvent(PublishLifecycleEventRequest),
    BuildCompleted,
}

pub struct BuildEventService<T>
where
    T: Send + Sync + 'static,
{
    pub write_channel: Arc<Mutex<Option<broadcast::Sender<BuildEventAction<T>>>>>,
    pub transform_fn:
        Arc<dyn Fn(&mut PublishBuildToolEventStreamRequest) -> Option<T> + Send + Sync>,
}

fn transform_queue_error_to_status() -> Status {
    Status::resource_exhausted("Exhausted queue when trying to publish message")
}

pub fn build_bazel_build_events_service() -> (
    BuildEventService<bazel_event::BazelBuildEvent>,
    Arc<Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>>,
    broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,
) {
    let (tx, rx) = broadcast::channel(256);
    let write_channel_arc = Arc::new(Mutex::new(Some(tx)));
    let server_instance = BuildEventService {
        write_channel: Arc::clone(&write_channel_arc),
        transform_fn: Arc::new(bazel_event::BazelBuildEvent::transform_from),
    };
    (server_instance, write_channel_arc, rx)
}

#[tonic::async_trait]
impl<T> PublishBuildEvent for BuildEventService<T>
where
    T: Send + Sync + Clone,
{
    type PublishBuildToolEventStreamStream = Pin<
        Box<
            dyn Stream<Item = Result<PublishBuildToolEventStreamResponse, Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn publish_build_tool_event_stream(
        &self,
        request: Request<tonic::Streaming<PublishBuildToolEventStreamRequest>>,
    ) -> Result<Response<Self::PublishBuildToolEventStreamStream>, Status> {
        let mut stream = request.into_inner();

        let sender_ref = {
            let e = Arc::clone(&self.write_channel);
            let m = e.lock().await;
            (*m).clone()
        };
        let cloned_v = sender_ref.clone();
        let second_writer = sender_ref.clone();
        let transform_fn = Arc::clone(&self.transform_fn);
        let output = async_stream::try_stream! {
            while let Some(inbound_evt) = stream.next().await {
                let mut inbound_evt = inbound_evt?;

                match inbound_evt.ordered_build_event.as_ref() {
                    Some(build_event) => {
                        let sequence_number = build_event.sequence_number;
                    yield PublishBuildToolEventStreamResponse {
                        stream_id: build_event.stream_id.clone(),
                        sequence_number: sequence_number
                    };

            }
                    None => ()
                };
                let transformed_data = (transform_fn)(&mut inbound_evt);

                if let Some(r) = transformed_data {
                    if let Some(tx) = cloned_v.as_ref() {
                        let tx2 = tx.clone();
                        tokio::spawn(async move {
                            let err = tx2.send(BuildEventAction::BuildEvent(r)).map_err(|_| transform_queue_error_to_status());
                            match err {
                                Ok(_) => (),
                                Err(e) =>
                                    error!("Error publishing to queue {}", e)
                            }
                        });
                    }
                }
            }


            if let Some(tx) = second_writer {
                tx.send(BuildEventAction::BuildCompleted).map_err(|_| transform_queue_error_to_status())?;
            }
            info!("Finished stream...");
        };

        Ok(Response::new(
            Box::pin(output) as Self::PublishBuildToolEventStreamStream
        ))
    }

    async fn publish_lifecycle_event(
        &self,
        request: tonic::Request<PublishLifecycleEventRequest>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let cloned_v = {
            let e = Arc::clone(&self.write_channel);
            let m = e.lock().await;
            (*m).clone()
        };

        if let Some(tx) = cloned_v {
            let inner = request.into_inner();
            info!("life cycle event: {:?}", inner);

            tx.send(BuildEventAction::LifecycleEvent(inner))
                .map_err(|_| transform_queue_error_to_status())?;
        }
        Ok(Response::new(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::prost::Message;

    use futures::future;
    use futures::future::FutureExt;
    use futures::stream;
    use futures::StreamExt;
    use pinky_swear::{Pinky, PinkySwear};
    use std::convert::TryFrom;
    use std::io::Read;
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::tempdir;
    use tokio::net::UnixListener;
    use tokio::net::UnixStream;
    use tokio::time;
    use tonic::transport::Server;
    use tonic::transport::{Endpoint, Uri};
    use tonic::Request;
    use tower::service_fn;

    fn load_proto(name: &str) -> Vec<PublishBuildToolEventStreamRequest> {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/build_events");
        d.push(name);

        let mut file = std::fs::File::open(d).expect("Expected to be able to open input test data");

        let mut data_vec = vec![];
        let _ = file
            .read_to_end(&mut data_vec)
            .expect("Expected to read file");

        let mut buf: &[u8] = &data_vec;
        let mut res_buf = vec![];

        while buf.len() > 0 {
            res_buf.push(
                PublishBuildToolEventStreamRequest::decode_length_delimited(&mut buf).unwrap(),
            );
        }
        res_buf
    }

    struct ServerStateHandler {
        _temp_dir_for_uds: tempfile::TempDir,
        completion_pinky: Pinky<()>,
        pub read_channel:
            Option<broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>>,
    }
    impl Drop for ServerStateHandler {
        fn drop(&mut self) {
            self.completion_pinky.swear(());
            // let the server shutdown gracefully before we cleanup the tempdir
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    use futures::TryStreamExt;
    use google::devtools::build::v1::publish_build_event_client;
    use google::devtools::build::v1::publish_build_event_server;

    async fn make_test_server() -> (
        ServerStateHandler,
        publish_build_event_client::PublishBuildEventClient<tonic::transport::channel::Channel>,
    ) {
        let uds_temp_dir = tempdir().unwrap();

        let path = uds_temp_dir.path().join("server_path");
        let path_copy = path.clone();
        println!("Path: {:?}", path);

        let (server_instance, _, rx) = build_bazel_build_events_service();

        let (promise, completion_pinky) = PinkySwear::<()>::new();
        let server_state = ServerStateHandler {
            _temp_dir_for_uds: uds_temp_dir,
            completion_pinky: completion_pinky,
            read_channel: Some(rx),
        };
        // let shutdown_promise =
        tokio::spawn(async {
            let mut uds = UnixListener::bind(path).expect("Should be able to setup unix listener");

            eprintln!("Starting server..");
            Server::builder()
                .add_service(publish_build_event_server::PublishBuildEventServer::new(
                    server_instance,
                ))
                .serve_with_incoming_shutdown(
                    uds.incoming().map_ok(crate::tokioext::unix::UnixStream),
                    promise,
                )
                .inspect(|x| println!("resolving future: {:?}", &x))
                .await
                .expect("Failed to start server")
        });

        time::delay_for(Duration::from_millis(5)).await;

        let endpoint: Endpoint =
            Endpoint::try_from("lttp://[::]:50051").expect("Can calculate endpoint");

        let channel: tonic::transport::channel::Channel = endpoint
            .connect_with_connector(service_fn(move |_: Uri| {
                let path_copy = path_copy.clone();
                // Connect to a Uds socket
                UnixStream::connect(path_copy)
            }))
            .await
            .expect("Connect to server");

        let client: publish_build_event_client::PublishBuildEventClient<
            tonic::transport::channel::Channel,
        > = publish_build_event_client::PublishBuildEventClient::new(channel);

        (server_state, client)
    }

    #[tokio::test]
    async fn test_no_op_build_stream() {
        let event_stream = load_proto("no_op_build.proto");
        let (mut state, mut client) = make_test_server().await;

        let stream = stream::iter(event_stream.clone());
        let ret_v = client
            .publish_build_tool_event_stream(Request::new(stream))
            .await
            .expect("service call should succeed")
            .into_inner();

        // need to exhaust the stream to ensure we complete the operation
        ret_v.for_each(|_| future::ready(())).await;

        let mut data_stream = vec![];
        let mut channel = state.read_channel.take().unwrap();

        tokio::spawn(async move {
            std::thread::sleep(Duration::from_millis(20));
            drop(state);
        });

        while let Ok(action) = channel.recv().await {
            match action {
                BuildEventAction::BuildCompleted => (),
                BuildEventAction::LifecycleEvent(_) => (),
                BuildEventAction::BuildEvent(msg) => {
                    data_stream.push(msg);
                }
            }
        }

        assert_eq!(event_stream.len(), data_stream.len());

        // Some known expected translations/rules and invariants:

        {
            use build_event_id::*;
            use build_event_stream::*;

            let expected = {
                // split these out since they block formatting :(
                let label_name =
                    String::from("//src/scala/com/github/johnynek/bazel_deps:settings_loader");
                bazel_event::BazelBuildEvent {
                    event: bazel_event::Evt::TargetCompleted(bazel_event::TargetCompletedEvt {
                        label: label_name,
                        success: true,
                        output_groups: vec![OutputGroup {
                            name: String::from("default"),
                            file_sets: vec![NamedSetOfFilesId {
                                id: String::from("16"),
                            }],
                        }],
                    }),
                }
            };
            assert_eq!(data_stream[80], expected);
        }
        // let mut idx = 0;
        // for e in data_stream {
        //     println!("{} -> {:?}", idx, e);
        //     idx = idx + 1;
        // }
        // assert_eq!(3, 5);
        // assert_eq!(event_stream, data_stream);
    }
}
