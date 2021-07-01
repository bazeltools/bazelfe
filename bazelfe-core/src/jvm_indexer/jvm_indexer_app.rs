use clap::Clap;
#[macro_use]
extern crate log;
use regex::Regex;

use lazy_static::lazy_static;

use std::path::PathBuf;
use std::time::Instant;

use std::env;
use tonic::transport::Server;

use bazelfe_protos::*;

use bazelfe_core::jvm_indexer::bazel_query::BazelQuery;
use bazelfe_core::{
    bazel_command_line_parser::ParsedCommandLine, build_events::hydrated_stream::HydratedInfo,
};
use bazelfe_core::{
    bazel_runner,
    hydrated_stream_processors::{
        event_stream_listener::EventStreamListener, index_new_results::IndexNewResults,
        BazelEventHandler,
    },
};
use bazelfe_core::{
    build_events::build_event_server::bazel_event,
    hydrated_stream_processors::target_completed_tracker::TargetCompletedTracker,
};
use bazelfe_core::{
    build_events::build_event_server::BuildEventAction, jvm_indexer::bazel_query::ExecuteResult,
};
use google::devtools::build::v1::publish_build_event_server::PublishBuildEventServer;
use rand::Rng;
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;
use tokio::sync::Mutex;

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

    /// Refresh bazel deps only
    #[clap(long)]
    refresh_bazel_deps_only: bool,

    /// Blacklist out these targets. Usually this matters when 3rdparty targets are poorly behaved and are not fully shaded
    /// and may include classes from other targets.
    #[clap(long)]
    blacklist_targets_from_index: Option<Vec<String>>,
}

#[derive(Clone, Debug)]
struct RuleQuery {
    pub kinds: Vec<String>,
    pub root: String,
}
fn build_rule_queries(
    allowed_rule_kinds: &Vec<String>,
    target_roots: &Vec<String>,
) -> Vec<RuleQuery> {
    let mut result = Vec::default();
    for target_root in target_roots {
        result.push(RuleQuery {
            kinds: allowed_rule_kinds.clone(),
            root: target_root.clone(),
        });
    }
    result
}
async fn spawn_bazel_attempt(
    sender_arc: &Arc<
        Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
    >,
    aes: &EventStreamListener,
    bes_port: u16,
    bazel_args: &ParsedCommandLine,
) -> bazel_runner::ExecuteResult {
    let (tx, rx) = async_channel::unbounded();
    let _ = {
        let mut locked = sender_arc.lock().await;
        *locked = Some(tx);
    };
    let error_stream = HydratedInfo::build_transformer(rx);

    let target_extracted_stream = aes.handle_stream(error_stream);

    let recv_task =
        tokio::spawn(async move { while let Ok(_) = target_extracted_stream.recv().await {} });
    let res = bazel_runner::execute_bazel_output_control(&bazel_args, bes_port, false)
        .await
        .expect("Internal errors should not occur invoking bazel.");

    let _ = {
        let mut locked = sender_arc.lock().await;
        locked.take();
    };

    recv_task.await.unwrap();
    res
}

async fn run_query_chunk<B: BazelQuery>(
    chunk: &[RuleQuery],
    bazel_query: &B,
    all_targets_to_use: &mut HashMap<String, HashSet<String>>,
    banned_roots: &HashSet<String>,
) -> ExecuteResult {
    let union_with_spaces_bytes = " union ".as_bytes();

    let merged = {
        let mut buffer = Vec::default();

        for rq in chunk {
            if !banned_roots.contains(&rq.root) {
                for kind in rq.kinds.iter() {
                    let x = format!("kind({}, {})", kind, rq.root);
                    if buffer.is_empty() {
                        buffer.write_all(&x.as_bytes()).unwrap();
                    } else {
                        buffer.write_all(&union_with_spaces_bytes).unwrap();
                        buffer.write_all(&x.as_bytes()).unwrap();
                    }
                }
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
                .or_insert(HashSet::default());
            entry.insert(entries[2].to_string());
        }
    }

    res
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

    let parsed_command_line = match bazelfe_core::bazel_command_line_parser::parse_bazel_command_line(&vec![opt.bazel_binary_path.to_string_lossy().to_string()]) {
        Ok(parsed_command_line) => {
            if parsed_command_line.is_action_option_set("bes_backend") {
                eprintln!("Bes backend already set, must exit since we can't add another safely.");
                std::process::exit(-1);
            }
            parsed_command_line
        }
        Err(cmd_line_parsing_failed) => {
            match cmd_line_parsing_failed {
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::MissingBazelPath => {
                    eprintln!("Missing bazel path, invalid command line arg supplied");
                    std::process::exit(-1);
                }
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::MissingArgToOption(o) => {
                    eprintln!("Arg parsing from bazelfe doesn't understand the args, missing an option to {}", o);
                    std::process::exit(-1);
                }
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::UnknownArgument(o) => {
                    eprintln!("Arg parsing from bazelfe doesn't understand the args, unknown option {}", o);
                    std::process::exit(-1);
                }
            }
        }
    };

    let mut rng = rand::thread_rng();
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    let mut running_refresh_mode = false;
    builder.target(pretty_env_logger::env_logger::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("warn,bazelfe_core::jvm_indexer=info,jvm_indexer=info");
    }
    builder.init();

    let target_blacklist = (&opt.blacklist_targets_from_index)
        .clone()
        .unwrap_or_default();

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

    let bazel_query =
        bazelfe_core::jvm_indexer::bazel_query::from_binary_path(&opt.bazel_binary_path);

    let bazel_deps_replacement_map: HashMap<String, String> = match &opt.bazel_deps_root {
        None => HashMap::default(),
        Some(bazel_deps_root) => {
            info!("Asked to find out information about a bazel_deps root for replacement, issuing queries");
            let targets_in_bazel_deps_root = bazel_query
                .execute(&vec![
                    String::from("query"),
                    format!("{}//3rdparty/jvm/...", bazel_deps_root),
                    String::from("--keep_going"),
                ])
                .await;

            info!("Graph query now starting");
            let bazel_deps_deps = bazel_query
                .execute(&vec![
                    String::from("query"),
                    String::from("--noimplicit_deps"),
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
                        if e.starts_with("@")
                            && (e.ends_with("//jar:jar")
                                || e.ends_with("//jar:file")
                                || e.ends_with("//jar"))
                        {
                            match e.split("//").next() {
                                Some(prefix) => {
                                    results_mapping.insert(
                                        format!("{}//jar:jar", prefix),
                                        bazel_dep.to_string(),
                                    );
                                    results_mapping.insert(
                                        format!("{}//jar:file", prefix),
                                        bazel_dep.to_string(),
                                    );
                                }
                                None => (),
                            }
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

    let union_with_spaces_bytes = " union ".as_bytes();

    let all_targets_to_use = if opt.refresh_bazel_deps_only {
        running_refresh_mode = true;
        let mut all_targets_to_use: HashMap<String, HashSet<String>> = HashMap::default();
        let merged = {
            let mut buffer = Vec::default();

            for x in bazel_deps_replacement_map.keys() {
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
                    .or_insert(HashSet::default());
                entry.insert(entries[2].to_string());
            }
        }
        all_targets_to_use
    } else {
        info!("Executing initial query to find all external repos in this bazel repository");

        let res = bazel_query
            .execute(&vec![String::from("query"), String::from("//external:*")])
            .await;

        let mut target_roots = vec![String::from("//...")];

        let mut blacklist_repos = vec![
            String::from("bazel-"),
            String::from("WORKSPACE"),
            String::from("bazel_tools"),
            String::from("remote_java_tools_linux"),
            String::from("jdk-default"),
            String::from("remote_java_tools_darwin"),
            String::from("remote_java_tools_windows"),
            String::from("remote_coverage_tools"),
            String::from("io_bazel_stardoc"),
            String::from("io_bazel"),
        ];

        if let Some(r) = parse_current_repo_name() {
            info!("Current repo name identified as {}", r);
            blacklist_repos.push(r);
        }
        blacklist_repos.extend(opt.blacklist_remote_roots.into_iter());

        for line in res.stdout.lines().into_iter() {
            if let Some(ln) = line.strip_prefix("//external:") {
                let mut ok = true;
                for root in &blacklist_repos {
                    if ln.starts_with(root) {
                        ok = false;
                    }
                }
                // Some externals are bind mounts
                if ln.contains("/") {
                    ok = false;
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

        let query_rule_attr_batch_size: usize = 10;
        info!("Extracting targets with an allowed rule kind, gives rise to {} total queries, we will union them to bazel in batches of size: {}", all_queries.len(), query_rule_attr_batch_size);

        let mut all_targets_to_use: HashMap<String, HashSet<String>> = HashMap::default();
        let mut processed_count = 0;
        let mut remaining_chunks: Vec<Vec<RuleQuery>> = all_queries
            .chunks(query_rule_attr_batch_size)
            .into_iter()
            .map(|e| e.to_vec())
            .collect();
        lazy_static! {
            static ref NO_REPO_REGEX: Regex =
                Regex::new(r#".*repository '(.*)' could not be resolved.$"#).unwrap();
                static ref OTHER_NO_REPO_REGEX: Regex =
                Regex::new(r#".*No such repository '(@.*)'"#).unwrap();

                static ref NOT_RESOLVABLE: Regex =
                Regex::new(r#".*error loading package '(@.*)//.*': Unable to find package for .*could not be resolved.$"#).unwrap();

                static ref NO_SUCH_FILE: Regex =
                    Regex::new(r#".*error loading package '(@.*)//.*': cannot load '.*': no such file$"#).unwrap();

                static ref NO_TARGETS_FOUND: Regex =
                    Regex::new(r#".*Skipping '(@.*)//...': no targets found beneath.*$"#).unwrap();

        }

        let mut global_banned_roots = HashSet::new();
        while let Some(current_chunk) = remaining_chunks.pop() {
            if current_chunk.len() > 0 {
                let res = run_query_chunk(
                    &current_chunk,
                    &bazel_query,
                    &mut all_targets_to_use,
                    &global_banned_roots,
                )
                .await;
                processed_count += 1;

                if res.exit_code != 0 {
                    if current_chunk.len() == 1 {
                        warn!(
                            "Unable to query into {}, may not have full target coverage.{}",
                            current_chunk[0].root, res.stderr
                        );
                    } else {
                        info!("Ran into some failures in query, will sub-divide to ensure we don't miss things. (was using chunk size ~ {}, will 1/4 that)", current_chunk.len());
                        let mut must_go_solo = HashSet::new();
                        let mut have_unmatched_line = false;
                        for ln in res.stderr.lines() {
                            let mut matched: bool = false;
                            if let Some(captures) = NO_REPO_REGEX.captures(&ln) {
                                let repo = captures.get(1).unwrap().as_str().to_string();
                                info!("Ignoring non existant repo: {}", repo);
                                global_banned_roots.insert(repo);
                                matched = true;
                            }

                            if let Some(captures) = OTHER_NO_REPO_REGEX.captures(&ln) {
                                let repo = captures.get(1).unwrap().as_str().to_string();
                                info!("Ignoring non existant repo: {}", repo);
                                global_banned_roots.insert(repo);
                                matched = true;
                            }

                            if let Some(captures) = NO_SUCH_FILE.captures(&ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }
                            if let Some(captures) = NOT_RESOLVABLE.captures(&ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }

                            if let Some(captures) = NO_TARGETS_FOUND.captures(&ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }

                            if !matched {
                                if ln.starts_with("ERROR:") {
                                    have_unmatched_line = true;
                                }
                            }
                        }

                        if have_unmatched_line {
                            let mut next_chunk_len = current_chunk.len() / 4;
                            if next_chunk_len == 0 {
                                next_chunk_len = 1;
                            };
                            let mut next_v: Vec<Vec<RuleQuery>> = Vec::default();

                            for e in current_chunk.iter() {
                                if must_go_solo.contains(&e.root) {
                                    next_v.push(vec![e.clone()]);
                                }
                            }
                            let next_v: Vec<Vec<RuleQuery>> = current_chunk
                                .chunks(next_chunk_len)
                                .into_iter()
                                .map(|e| {
                                    e.to_vec()
                                        .into_iter()
                                        .filter(|e| {
                                            !must_go_solo.contains(&e.root)
                                                && !global_banned_roots.contains(&e.root)
                                        })
                                        .collect()
                                })
                                .collect();
                            remaining_chunks.extend(next_v.into_iter());
                        }
                    }
                } else {
                    info!(
                        "After {} bazel query calls, found {} matching targets",
                        processed_count,
                        all_targets_to_use.values().fold(0, |acc, e| acc + e.len())
                    );
                }
            }
        }
        all_targets_to_use
    };

    info!("Found targets");
    let mut all_found_targets = HashSet::new();
    for (k, v) in all_targets_to_use.iter() {
        let spaces = 70 - k.len();
        let space_section = std::iter::repeat(" ").take(spaces).collect::<String>();
        info!("{}{}{}", k, space_section, v.len());
        for e in v.iter() {
            all_found_targets.insert(e.clone());
        }
    }

    let index_table = if running_refresh_mode && opt.index_output_location.exists() {
        let mut src_f = std::fs::File::open(&opt.index_output_location).unwrap();
        bazelfe_core::index_table::IndexTable::read(&mut src_f)
    } else {
        bazelfe_core::index_table::IndexTable::default()
    };

    for e in target_blacklist {
        index_table.add_target_to_blacklist(e).await
    }

    let target_completed_tracker = TargetCompletedTracker::new(all_found_targets);

    let processors: Vec<Arc<dyn BazelEventHandler>> = vec![
        Arc::new(IndexNewResults::new(index_table.clone())),
        Arc::new(target_completed_tracker.clone()),
    ];
    let aes = EventStreamListener::new(processors);

    let popularity_data =
        bazelfe_core::jvm_indexer::popularity_parser::build_popularity_map().await;

    for (k, v) in popularity_data {
        index_table.set_popularity_str(k, v as u16).await
    }

    for (k, v) in bazel_deps_replacement_map {
        index_table.add_transformation_mapping(k, v).await;
    }

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

    async fn run_bazel(
        bes_port: u16,
        sender_arc: Arc<
            Mutex<Option<async_channel::Sender<BuildEventAction<bazel_event::BazelBuildEvent>>>>,
        >,
        parsed_command_line: &ParsedCommandLine,
        aes: &EventStreamListener,
        batch_idx: usize,
        chunk: &mut Vec<String>,
        target_completed_tracker: &TargetCompletedTracker,
    ) {
        let batch_idx = batch_idx;
        let batch_start_time = Instant::now();

        let mut parsed_command_line = parsed_command_line.clone();
        parsed_command_line.set_action(Some(
            bazelfe_core::bazel_command_line_parser::Action::BuiltIn(
                bazelfe_core::bazel_command_line_parser::BuiltInAction::Build,
            ),
        ));

        parsed_command_line.add_action_option_if_unset(
            bazelfe_core::bazel_command_line_parser::BazelOption::BooleanOption(
                String::from("keep_going"),
                true,
            ),
        );

        parsed_command_line.remaining_args.extend(chunk.drain(..));
        let bazel_result =
            spawn_bazel_attempt(&sender_arc, &aes, bes_port, &parsed_command_line).await;

        let remaining_targets = target_completed_tracker.expected_targets.lock().await.len();

        info!(
            "Batch {} had exit code: {} after {} seconds, estimated from remaining targets: {}",
            batch_idx,
            bazel_result.exit_code,
            batch_start_time.elapsed().as_secs(),
            remaining_targets
        );
    }

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
                &parsed_command_line,
                &aes,
                batch_idx,
                &mut batch_elements,
                &target_completed_tracker,
            )
            .await;
            batch_idx += 1;
        }
        batch_elements.push(cur);
    }
    run_bazel(
        bes_port,
        Arc::clone(&sender_arc),
        &parsed_command_line,
        &aes,
        batch_idx,
        &mut batch_elements,
        &target_completed_tracker,
    )
    .await;

    info!("Building a target popularity map");

    info!("Writing out index data");

    let mut file = std::fs::File::create(&opt.index_output_location).unwrap();

    index_table.write(&mut file).await;

    // When operating on bazel deps the number of targets we feed into build isn't filtered to the particular types
    // this means we wind up not building everything since some don't show up in the BEP.
    if !opt.refresh_bazel_deps_only {
        let tt_map = target_completed_tracker.expected_targets.lock().await;
        for e in tt_map.iter() {
            if e.starts_with("//") {
                println!("Didn't build target: {}", e);
            }
        }
    }

    Ok(())
}
