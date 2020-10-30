#[macro_use]
extern crate log;

use clap::{AppSettings, Clap};
use std::{collections::HashMap, path::PathBuf};

use std::env;
use tonic::transport::Server;

use bazelfe_protos::*;
use std::ffi::OsString;

use bazelfe_core::build_events::build_event_server::bazel_event;
use bazelfe_core::build_events::build_event_server::BuildEventAction;
use bazelfe_core::build_events::hydrated_stream::HydratedInfo;
use bazelfe_core::buildozer_driver;
use bazelfe_core::{
    bazel_runner,
    hydrated_stream_processors::{
        event_stream_listener::EventStreamListener,
        index_new_results::IndexNewResults,
        process_bazel_failures::{ProcessBazelFailures, TargetStory, TargetStoryAction},
        BazelEventHandler,
    },
};
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

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

struct ProcessorActivity {
    pub jvm_segments_indexed: u32,
    pub actions_taken: u32,
    pub target_story_actions: HashMap<String, Vec<TargetStory>>,
}
impl ProcessorActivity {
    pub fn merge(&mut self, o: &ProcessorActivity) {
        for (_, story_entries) in o.target_story_actions.iter() {
            for story_entry in story_entries {
                match story_entry.action {
                    TargetStoryAction::Success => {
                        self.target_story_actions.remove(&story_entry.target);
                    }
                    _ => {
                        match self.target_story_actions.get_mut(&story_entry.target) {
                            None => {
                                self.target_story_actions
                                    .insert(story_entry.target.clone(), vec![story_entry.clone()]);
                            }
                            Some(existing) => existing.push(story_entry.clone()),
                        };
                    }
                }
            }
        }

        self.jvm_segments_indexed += o.jvm_segments_indexed;
        self.actions_taken += o.actions_taken;
    }
}
impl Default for ProcessorActivity {
    fn default() -> Self {
        ProcessorActivity {
            jvm_segments_indexed: 0,
            actions_taken: 0,
            target_story_actions: HashMap::new(),
        }
    }
}
async fn spawn_bazel_attempt(
    sender_arc: &Arc<
        Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
    >,
    aes: &EventStreamListener,
    bes_port: u16,
    passthrough_args: &Vec<String>,
) -> (ProcessorActivity, bazel_runner::ExecuteResult) {
    let (tx, rx) = async_channel::unbounded();
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };
    let error_stream = HydratedInfo::build_transformer(rx);

    let target_extracted_stream = aes.handle_stream(error_stream);

    let results_data = Arc::new(RwLock::new(None));
    let r_data = Arc::clone(&results_data);
    let recv_task = tokio::spawn(async move {
        let mut guard = r_data.write().await;

        let mut jvm_segments_indexed = 0;
        let mut actions_taken: u32 = 0;
        let mut target_story_actions = HashMap::new();

        while let Ok(action) = target_extracted_stream.recv().await {
            match action {
                bazelfe_core::hydrated_stream_processors::BuildEventResponse::ProcessedBuildFailures(pbf) =>  {
                    let current_updates: u32 = pbf.target_story_entries.iter().map (|e| {
                        match e.action {
                            TargetStoryAction::Success => 0,
                            _ => 1
                        }
                    }).sum();
                    actions_taken += current_updates;
                    for story_entry in pbf.target_story_entries {
                        match target_story_actions.get_mut(&story_entry.target) {
                            None => {
                                target_story_actions.insert(story_entry.target.clone(), vec![story_entry]);
                            }
                            Some(existing) =>
                            existing.push(story_entry)
                        };
                        }
                    }
                bazelfe_core::hydrated_stream_processors::BuildEventResponse::IndexedResults(ir) => {
                    jvm_segments_indexed += ir.jvm_segments_indexed
                }
            }
        }

        *guard = Some(ProcessorActivity {
            jvm_segments_indexed: jvm_segments_indexed,
            actions_taken: actions_taken,
            target_story_actions: target_story_actions,
        });
    });

    let res = bazel_runner::execute_bazel(passthrough_args.clone(), bes_port).await;

    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };

    recv_task.await.unwrap();
    let r = results_data.write().await.take().unwrap();
    (r, res)
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
    builder.target(pretty_env_logger::env_logger::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("warn,bazelfe_core=info,bazel_runner=info");
    }
    builder.init();

    bazel_runner::register_ctrlc_handler();

    debug!("Loading index..");
    let index_table = match &opt.index_input_location {
        Some(p) => {
            if p.exists() {
                let mut src_f = std::fs::File::open(p).unwrap();
                bazelfe_core::index_table::IndexTable::read(&mut src_f)
            } else {
                bazelfe_core::index_table::IndexTable::new()
            }
        }
        None => bazelfe_core::index_table::IndexTable::new(),
    };

    debug!("Index loading complete..");

    let processors: Vec<Box<dyn BazelEventHandler>> = vec![
        Box::new(ProcessBazelFailures::new(
            index_table.clone(),
            buildozer_driver::from_binary_path(opt.buildozer_path),
        )),
        Box::new(IndexNewResults::new(index_table.clone())),
    ];
    let aes = EventStreamListener::new(processors);

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
    debug!("Services listening on {}", addr);

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

    let mut running_total = ProcessorActivity::default();
    let mut final_exit_code = 0;
    while attempts < 15 {
        attempts += 1;

        let (processor_activity, bazel_result) =
            spawn_bazel_attempt(&sender_arc, &aes, bes_port, &passthrough_args).await;
        running_total.merge(&processor_activity);
        final_exit_code = bazel_result.exit_code;
        if bazel_result.exit_code == 0 || processor_activity.actions_taken == 0 {
            break;
        }
    }

    // we should be very quiet if the build is successful/we added nothing.
    if attempts > 1 {
        eprintln!("--------------------Bazel Runner Report--------------------");
        eprintln!("Bazel build attempts: {}", attempts);
        eprintln!("Actions taken: {}", running_total.actions_taken);
        eprintln!(
            "Jvm fragments (classes/packages) added to index: {}",
            running_total.jvm_segments_indexed
        );
        if final_exit_code != 0 && running_total.target_story_actions.len() > 0 {
            eprintln!(
                "\nBuild still failed. Active stories about failed targets/what we've tried:"
            );
            let mut v: Vec<(String, Vec<TargetStory>)> =
                running_total.target_story_actions.into_iter().collect();
            v.sort_by_key(|k| k.0.clone());
            for (label, mut story_entries) in v.into_iter() {
                eprintln!("Target: {}", label);
                story_entries.sort_by_key(|e| e.when.clone());
                for entry in story_entries.into_iter() {
                    match entry.action {
                        TargetStoryAction::AddedDependency { added_what, why } => {
                            eprintln!("\tAdded Dependency {}, because: {}", added_what, why);
                        }
                        TargetStoryAction::RemovedDependency { removed_what, why } => {
                            eprintln!("\tRemoved Dependency {}, because: {}", removed_what, why);
                        }
                        TargetStoryAction::Success => panic!("Shouldn't have a success item here"),
                    }
                }
            }
        }
        eprintln!("------------------------------------------------------------\n");
    }

    if index_table.is_mutated() {
        debug!("Writing out index file...");

        if let Some(e) = &opt.index_input_location {
            if let Some(parent) = e.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            let mut file = std::fs::File::create(&e).unwrap();
            index_table.write(&mut file).await
        }
        debug!("Index write complete.");
    }
    std::process::exit(final_exit_code);
}
