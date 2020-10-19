use clap::{Clap};
#[macro_use]
extern crate log;
use regex::Regex;

use lazy_static::lazy_static;

use std::path::PathBuf;
use std::time::Instant;

use std::env;
use std::sync::atomic::Ordering;
use tonic::transport::Server;

use bazelfe_protos::*;

use bazelfe_core::bazel_runner;
use bazelfe_core::build_events::build_event_server::bazel_event;
use bazelfe_core::build_events::build_event_server::BuildEventAction;
use bazelfe_core::build_events::hydrated_stream::HydratedInfo;
use bazelfe_core::jvm_indexer::bazel_query::BazelQuery;
use dashmap::{DashMap};
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    /// Optional if you have some restrictions/needs where the server bazel will connect to should bind
    /// default to a random port on 127.0.0.1
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    /// Where to find the bazel to invoke, if its just on your path `which bazel` could be passed here.
    #[clap(long, parse(from_os_str))]
    bazel_binary_path: PathBuf,

    /// Where the output index should be stored
    #[clap(long, env = "INDEX_OUTPUT_LOCATION", parse(from_os_str))]
    index_output_location: PathBuf,

    /// Paths to ignore for dependencies, a good value here when working with scala code is `io_bazel_rules_scala`
    #[clap(long)]
    blacklist_remote_roots: Vec<String>,

    /// Extra rules other than the default java,scala,java proto, scala proto rules to allow jars from
    #[clap(long)]
    extra_allowed_rule_kinds: Option<Vec<String>>,

    /// An optional bazel deps root, something like `@third_party_jvm`
    /// when present we will use this root to try calculate the mapping of a bazel deps
    /// to underlying raw jar. Then apply that reverse mapping so missing dependencies/the index built
    /// will use the bazel deps entry rather than the raw jar.
    #[clap(long)]
    bazel_deps_root: Option<String>,
}

fn build_rule_queries(allowed_rule_kinds: &Vec<String>, target_roots: &Vec<String>) -> Vec<String> {
    let mut result = Vec::default();
    for target_root in target_roots {
        for allowed_kind in allowed_rule_kinds.iter() {
            result.push(format!("kind({}, {})", allowed_kind, target_root))
        }
    }
    result
}
async fn spawn_bazel_attempt(
    sender_arc: &Arc<
        Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
    >,
    aes: &bazelfe_core::jvm_indexer::indexer_action_event_stream::IndexerActionEventStream,
    bes_port: u16,
    bazel_args: &Vec<String>,
    index_map: Arc<DashMap<String, Vec<String>>>,
) -> (usize, bazel_runner::ExecuteResult) {
    let (tx, rx) = broadcast::channel(8192);
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };
    let error_stream = HydratedInfo::build_transformer(rx);

    let mut target_extracted_stream = aes.build_action_pipeline(error_stream, index_map);

    let actions_completed: Arc<std::sync::atomic::AtomicUsize> =
        Arc::new(std::sync::atomic::AtomicUsize::new(0));

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
    let res = bazel_runner::execute_bazel_output_control(bazel_args.clone(), bes_port, false).await;

    info!("Bazel completed with state: {:?}", res);
    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };

    recv_task.await.unwrap();
    info!("Receive task done");
    (actions_completed.fetch_add(0, Ordering::Relaxed), res)
}

fn parse_current_repo_name() -> Option<String> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"^\s*workspace\(\s*name\s*=\s*("|')\s*([A-Za-z0-9_-]*)("|').*$"#).unwrap();
    }

    let workspace_path = PathBuf::from("WORKSPACE");
    if workspace_path.exists() {
        let workspace_content = std::fs::read_to_string(workspace_path).unwrap();
        let ln = workspace_content
            .lines()
            .filter(|e| e.starts_with("workspace("))
            .next();
        if let Some(line) = ln {
            if let Some(captures) = RE.captures(&line) {
                return Some(String::from(captures.get(2).unwrap().as_str()));
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();
    let mut rng = rand::thread_rng();
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    builder.target(env_logger::fmt::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("warn,bazelfe_core::jvm_indexer=info,jvm_indexer=info");
        // builder.parse_filters("info");
    }
    builder.init();
    let bazel_binary_path: String = (&opt.bazel_binary_path.to_str().unwrap()).to_string();

    let allowed_rule_kinds: Vec<String> = vec![
        "java_library",
        "java_import",
        "scala_import",
        "scala_library",
        "scala_proto_library",
        "scala_macro_library",
        "java_proto_library",
        "_java_grpc_library",
    ]
    .into_iter()
    .map(|e| e.to_string())
    .chain(opt.extra_allowed_rule_kinds.unwrap_or_default().into_iter())
    .collect();

    let bazel_query = bazelfe_core::jvm_indexer::bazel_query::from_binary_path(opt.bazel_binary_path);

    let bazel_deps_replacement_map: HashMap<String, String> = match &opt.bazel_deps_root {
        None => HashMap::default(),
        Some(bazel_deps_root) => {
            let targets_in_bazel_deps_root = bazel_query
                .execute(&vec![
                    String::from("query"),
                    format!("{}//3rdparty/jvm/...", bazel_deps_root),
                    String::from("--keep_going"),
                ])
                .await;

            let bazel_deps_deps = bazel_query
                .execute(&vec![
                    String::from("query"),
                    format!("deps({}//3rdparty/jvm/...)", bazel_deps_root),
                    String::from("--output"),
                    String::from("graph"),
                    String::from("--keep_going"),
                ])
                .await;

            let bazel_deps = {
                let mut bazel_deps = HashSet::new();
                for ln in targets_in_bazel_deps_root.stdout.lines().into_iter() {
                    bazel_deps.insert(ln);
                }
                bazel_deps
            };
            let mut mapping = HashMap::new();
            for ln in bazel_deps_deps.stdout.lines().into_iter() {
                if ln.contains(" -> ") {
                    let elements: Vec<&str> = ln.split(" -> ").collect();
                    if elements.len() > 1 {
                        let src = elements[0].trim();
                        let dest = elements[1].trim();

                        let e = mapping
                            .entry(src.replace("\"", "").to_string())
                            .or_insert(Vec::default());
                        e.push(dest.replace("\"", ""));
                    }
                }
            }

            let mut results_mapping = HashMap::new();
            for bazel_dep in bazel_deps {
                if let Some(values) = mapping.get(&bazel_dep.to_string()) {
                    let mut values = values.clone();
                    while !values.is_empty() {
                        let e = values.pop().unwrap();
                        if e.starts_with("@") {
                            results_mapping.insert(e, bazel_dep.to_string());
                        } else if e.starts_with("//external") {
                            if let Some(r) = mapping.get(&e) {
                                values.extend(r.clone().into_iter());
                            }
                        }
                    }
                }
            }
            results_mapping
        }
    };

    info!("Executing initial query to find all external repos in this bazel repository");

    let res = bazel_query
        .execute(&vec![String::from("query"), String::from("//external:*")])
        .await;

    let mut target_roots = vec![String::from("//...")];

    let mut blacklist_repos = vec![String::from("bazel-"), String::from("WORKSPACE")];
    if let Some(r) = parse_current_repo_name() {
        info!("Current repo name identified as {}", r);
        blacklist_repos.push(r);
    }
    blacklist_repos.extend(opt.blacklist_remote_roots.into_iter());

    for line in res.stdout.lines().into_iter() {
        if let Some(ln) = line.strip_prefix("//external:") {
            let mut ok = true;
            for root in &blacklist_repos {
                if ln.contains(root) {
                    ok = false;
                }
            }

            if ok {
                target_roots.push(format!("@{}//...", ln));
            }
        }
    }

    if res.exit_code != 0 {
        info!("The bazel query returned something other than exit code zero, this unfortunately can often happen, so we will continue with the data received. We have identified {} target roots", target_roots.len());
    } else {
        info!("We have identified {} target roots", target_roots.len());
    }

    let all_queries = build_rule_queries(&allowed_rule_kinds, &target_roots);

    let query_rule_attr_batch_size: usize = 2000;
    info!("Extracting targets with an allowed rule kind, gives rise to {} total queries, we will union them to bazel in batches of size: {}", all_queries.len(), query_rule_attr_batch_size);

    let union_with_spaces_bytes = " union ".as_bytes();

    let mut all_targets_to_use: HashMap<String, Vec<String>> = HashMap::default();
    let mut processed_count = 0;
    for chunk in all_queries.chunks(query_rule_attr_batch_size) {
        let merged = {
            let mut buffer = Vec::default();

            for x in chunk {
                if buffer.is_empty() {
                    buffer.write_all(&x.as_bytes()).unwrap();
                } else {
                    buffer.write_all(&union_with_spaces_bytes).unwrap();
                    buffer.write_all(&x.as_bytes()).unwrap();
                }
            }
            String::from_utf8(buffer).unwrap()
        };
        let res = bazel_query
            .execute(&vec![
                String::from("query"),
                String::from("--keep_going"),
                String::from("--noimplicit_deps"),
                String::from("--output"),
                String::from("label_kind"),
                merged,
            ])
            .await;

        for ln in res.stdout.lines() {
            let entries: Vec<&str> = ln.split_whitespace().collect();
            if entries.len() == 3 {
                let entry = all_targets_to_use
                    .entry(entries[0].to_string())
                    .or_insert(Vec::default());
                entry.push(entries[2].to_string());
            }
            // all_targets_to_use.push(ln.to_string());
        }
        processed_count += chunk.len();
        info!(
            "After {} queries, found {} matching targets",
            processed_count,
            all_targets_to_use.values().fold(0, |acc, e| acc + e.len())
        );
    }

    info!("Found targets");
    for (k, v) in all_targets_to_use.iter() {
        let spaces = 70 - k.len();
        let space_section = std::iter::repeat(" ").take(spaces).collect::<String>();
        info!("{}{}{}", k, space_section, v.len());
    }

    let aes = bazelfe_core::jvm_indexer::indexer_action_event_stream::IndexerActionEventStream::new(
        allowed_rule_kinds,
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

    let compile_batch_size: usize = 1000;
    info!(
        "About to start building targets, will occur in batches of size: {}",
        compile_batch_size
    );

    let results_map: Arc<DashMap<String, Vec<String>>> = Arc::new(DashMap::new());
    async fn run_bazel(
        bes_port: u16,
        sender_arc: Arc<
            Mutex<Option<broadcast::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
        >,
        bazel_binary_path: String,
        aes: &bazelfe_core::jvm_indexer::indexer_action_event_stream::IndexerActionEventStream,
        batch_idx: usize,
        chunk: &mut Vec<String>,
        results_map: Arc<DashMap<String, Vec<String>>>,
    ) {
        let batch_idx = batch_idx;
        let batch_start_time = Instant::now();
        let mut current_args: Vec<String> = vec![
            bazel_binary_path,
            String::from("build"),
            String::from("--keep_going"),
        ];
        current_args.extend(chunk.drain(..));
        let (_num_classes_found, bazel_result) =
            spawn_bazel_attempt(&sender_arc, &aes, bes_port, &current_args, results_map).await;
        info!(
            "Batch {} had exit code: {} after {} seconds",
            batch_idx,
            bazel_result.exit_code,
            batch_start_time.elapsed().as_secs()
        );
    };

    let mut batch_idx = 0;
    let mut batch_elements = Vec::default();
    for cur in all_targets_to_use
        .into_iter()
        .into_iter()
        .flat_map(|(_, e)| e.into_iter())
    {
        if batch_elements.len() >= compile_batch_size {
            run_bazel(
                bes_port,
                Arc::clone(&sender_arc),
                bazel_binary_path.clone(),
                &aes,
                batch_idx,
                &mut batch_elements,
                Arc::clone(&results_map),
            )
            .await;
            batch_idx += 1;
        }
        batch_elements.push(cur);
    }
    run_bazel(
        bes_port,
        Arc::clone(&sender_arc),
        bazel_binary_path.clone(),
        &aes,
        batch_idx,
        &mut batch_elements,
        Arc::clone(&results_map),
    )
    .await;

    info!("Building a target popularity map");
    let ret = bazelfe_core::jvm_indexer::popularity_parser::build_popularity_map();

    let mut reverse_hashmap = HashMap::new();

    info!("Building results map, and injecting popularity data");
    for kv in results_map.iter() {
        let key = kv.key();
        let value = kv.value();
        let re = bazel_deps_replacement_map.get(key).unwrap_or(key).clone();
        let freq: usize = ret.get(&re).unwrap_or(&0).clone();
        for inner_v in value {
            let v = reverse_hashmap.entry(inner_v.clone()).or_insert(vec![]);
            v.push((freq, re.clone()))
        }
    }

    let res_vec = {
        let mut v1: Vec<(String, Vec<(usize, String)>)> = reverse_hashmap.into_iter().collect();

        v1.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        v1
    };

    info!("Writing out index data");
    let mut file = std::fs::File::create(&opt.index_output_location).unwrap();

    for (k, mut innerv) in res_vec.into_iter() {
        file.write_all(k.as_bytes())?;
        file.write_all("\t".as_bytes())?;
        // reverse sort
        innerv.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut idx = 0;
        for (cnt, v) in innerv.into_iter() {
            if idx > 0 {
                file.write_all(",".as_bytes()).unwrap();
            }
            file.write_all(format!("{}:{}", cnt, v).as_bytes()).unwrap();
            idx += 1;
        }
        file.write_all("\n".as_bytes()).unwrap();
    }

    Ok(())
}
