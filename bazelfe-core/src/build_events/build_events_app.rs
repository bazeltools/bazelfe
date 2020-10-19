#[macro_use]
extern crate log;

use clap::Clap;

use std::{env, sync::Arc};
use tonic::transport::Server;

use ::prost::Message;
use bazelfe_core::build_events::build_event_server::{BuildEventAction, BuildEventService};
use bazelfe_protos::*;
use tokio::{prelude::*, sync::Mutex};

use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use google::devtools::build::v1::PublishBuildToolEventStreamRequest;
use tokio::sync::broadcast;

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    #[clap(name = "BIND_ADDRESS")]
    bind_address: Option<String>,
}

fn transform_fn(e: &mut PublishBuildToolEventStreamRequest) -> Option<Vec<u8>> {
    let mut buf = vec![];
    e.encode_length_delimited(&mut buf).unwrap();
    Some(buf)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    let addr = opt
        .bind_address
        .map(|s| s.to_owned())
        .or(env::var("BIND_ADDRESS").ok())
        .unwrap_or_else(|| "127.0.0.1:50051".into())
        .parse()
        .expect("can't parse BIND_ADDRESS variable");

    info!("Services listening on {}", addr);

    let (tx, mut rx) = broadcast::channel(32);

    let greeter = BuildEventService {
        write_channel: Arc::new(Mutex::new(Some(tx))),
        transform_fn: std::sync::Arc::new(transform_fn),
    };

    tokio::spawn(async move {
        let mut file: Option<tokio::fs::File> = None;
        let mut idx: u32 = 0;
        while let Ok(action) = rx.recv().await {
            match action {
                BuildEventAction::BuildCompleted => {
                    let _ = file.take();
                    ()
                }
                BuildEventAction::LifecycleEvent(_) => (),
                BuildEventAction::BuildEvent(msg) => {
                    match file {
                        None => {
                            idx = idx + 1;
                            let f = tokio::fs::File::create(format!("build_events_{}.proto", idx))
                                .await
                                .unwrap();
                            file = Some(f);
                        }
                        Some(_) => (),
                    };

                    if let Some(ref mut f) = file {
                        let _res = f.write(&msg).await.unwrap();
                    }
                }
            }
        }
    });

    Server::builder()
        .add_service(PublishBuildEventServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
