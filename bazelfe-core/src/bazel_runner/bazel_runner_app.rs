#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};
use std::path::PathBuf;

use std::env;
use std::sync::atomic::Ordering;
use tonic::transport::Server;

use bazelfe_protos::*;
use std::ffi::OsString;

use bazelfe_core::bazel_runner;
use bazelfe_core::build_events::build_event_server::bazel_event;
use bazelfe_core::build_events::build_event_server::BuildEventAction;
use bazelfe_core::build_events::hydrated_stream::HydratedInfo;
use bazelfe_core::buildozer_driver;
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clap, Debug)]
#[clap(name = "basic", setting = AppSettings::TrailingVarArg)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long, env = "INDEX_INPUT_LOCATION", parse(from_os_str))]
    index_input_location: Option<PathBuf>,

    #[clap(long, env = "BUILDOZER_PATH", parse(from_os_str))]
    buildozer_path: PathBuf,

    #[clap(required = true, min_values = 1)]
    passthrough_args: Vec<String>,
}
// BuildEventService<bazel_event::BazelBuildEvent>,
// Arc<Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>>,
// broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,

async fn spawn_bazel_attempt<T>(
    sender_arc: &Arc<
        Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
    >,
    aes: &bazel_runner::action_event_stream::ActionEventStream<T>,
    bes_port: u16,
    passthrough_args: &Vec<String>,
) -> (u32, bazel_runner::ExecuteResult)
where
    T: bazelfe_core::buildozer_driver::Buildozer + Send + Clone + Sync + 'static,
{
    let (tx, rx) = broadcast::channel(8192);
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };
    let error_stream = HydratedInfo::build_transformer(rx);

    let mut target_extracted_stream = aes.build_action_pipeline(error_stream);

    let actions_completed: Arc<std::sync::atomic::AtomicU32> =
        Arc::new(std::sync::atomic::AtomicU32::new(0));

    let recv_ver = Arc::clone(&actions_completed);
    let recv_task = tokio::spawn(async move {
        while let Some(action) = target_extracted_stream.recv().await {
            match action {
                None => (),
                Some(err_info) => {
                    recv_ver.fetch_add(err_info, Ordering::Relaxed);
                }
            }
        }
    });
    let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;

    info!("Bazel completed with state: {:?}", res);
    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };

    recv_task.await.unwrap();
    info!("Receive task done");
    (actions_completed.fetch_add(0, Ordering::Relaxed), res)
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    // If someone is using a bes backend we need to nope out so we don't conflict.
    // This also means our other tools can call in using our same utilities
    // with this already set to make this app passthrough
    if opt
        .passthrough_args
        .contains(&String::from("--bes_backend"))
    {
        let application: OsString = opt
            .passthrough_args
            .first()
            .map(|a| {
                let a: String = a.clone().into();
                a
            })
            .expect("Should have had at least one arg the bazel process itself.")
            .into();

        let remaining_args: Vec<OsString> = opt
            .passthrough_args
            .iter()
            .skip(1)
            .map(|str_ref| {
                let a: String = str_ref.clone().into();
                let a: OsString = a.into();
                a
            })
            .collect();

        let resp = ::exec::Command::new(application)
            .args(&remaining_args)
            .exec();
        panic!("Should be unreachable: {:#?}", resp);
    }

    let mut rng = rand::thread_rng();
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    builder.target(env_logger::fmt::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    }

    builder.init();

    bazel_runner::register_ctrlc_handler();

    let aes = bazel_runner::action_event_stream::ActionEventStream::new(
        opt.index_input_location,
        buildozer_driver::from_binary_path(opt.buildozer_path),
    );

    let default_port = {
        let rand_v: u16 = rng.gen();
        40000 + (rand_v % 3000)
    };

    let addr: std::net::SocketAddr = opt
        .bind_address
        .map(|s| s.to_owned())
        .or(env::var("BIND_ADDRESS").ok())
        .unwrap_or_else(|| format!("127.0.0.1:{}", default_port).into())
        .parse()
        .expect("can't parse BIND_ADDRESS variable");

    let passthrough_args = opt.passthrough_args.clone();
    info!("Services listening on {}", addr);

    let (bes, sender_arc, _) =
    bazelfe_core::build_events::build_event_server::build_bazel_build_events_service();

    let bes_port: u16 = addr.port();

    let _service_fut = tokio::spawn(async move {
        Server::builder()
            .add_service(PublishBuildEventServer::new(bes))
            .serve(addr)
            .await
            .unwrap();
    });

    let mut attempts: u16 = 0;

    let mut final_exit_code = 0;
    while attempts < 15 {
        let (actions_corrected, bazel_result) =
            spawn_bazel_attempt(&sender_arc, &aes, bes_port, &passthrough_args).await;
        final_exit_code = bazel_result.exit_code;
        if bazel_result.exit_code == 0 || actions_corrected == 0 {
            break;
        }
        attempts += 1;
    }

    info!("Attempts/build cycles: {:?}", attempts);
    std::process::exit(final_exit_code);
}
