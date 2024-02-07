use bazelfe_bazel_wrapper::bazel_subprocess_wrapper::{BazelWrapper, BazelWrapperBuilder};

use bazelfe_bazel_wrapper::bep::target_completed_tracker::TargetCompletedTracker;

use bazelfe_core::config::load_config_file;
use bazelfe_core::hydrated_stream_processors::index_new_results::IndexNewResults;
use bazelfe_core::hydrated_stream_processors::BuildEventResponse;
use clap::Parser;
#[macro_use]
extern crate log;
use regex::Regex;

use lazy_static::lazy_static;

use std::path::PathBuf;
use std::time::Instant;

use std::env;

use bazelfe_bazel_wrapper::bazel_command_line_parser::parse_bazel_command_line;
use bazelfe_bazel_wrapper::bazel_command_line_parser::{self, ParsedCommandLine};
use bazelfe_core::jvm_indexer::bazel_query::BazelQuery;

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[clap(name = "basic")]
struct Opt {
    /// Optional if you have some restrictions/needs where the server bazel will connect to should bind
    /// default to a random port on 127.0.0.1
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    /// Where to find the bazel to invoke, if its just on your path `which bazel` could be passed here.
    #[clap(long)]
    bazel_binary_path: PathBuf,

    #[clap(long)]
    config: Option<String>,

    /// Where the output index should be stored
    #[clap(long, env = "INDEX_OUTPUT_LOCATION")]
    index_output_location: PathBuf,

    /// Paths to ignore for dependencies, a good value here when working with scala code is `io_bazel_rules_scala`
    #[clap(long)]
    blacklist_remote_roots: Vec<String>,

    /// Extra rules other than the default java,scala,java proto, scala proto rules to allow jars from
    #[clap(long)]
    extra_allowed_rule_kinds: Option<Vec<String>>,

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
fn build_rule_queries(allowed_rule_kinds: &[String], target_roots: &[String]) -> Vec<RuleQuery> {
    let mut result = Vec::default();
    for target_root in target_roots {
        result.push(RuleQuery {
            kinds: allowed_rule_kinds.to_vec(),
            root: target_root.clone(),
        });
    }
    result
}

async fn run_query_chunk<B: BazelQuery>(
    chunk: &[RuleQuery],
    bazel_query: &B,
    all_targets_to_use: &mut HashMap<String, HashSet<String>>,
    banned_roots: &HashSet<String>,
) -> bazelfe_core::jvm_indexer::bazel_query::ExecuteResult {
    let union_with_spaces_bytes = " union ".as_bytes();

    let merged = {
        let mut buffer = Vec::default();

        for rq in chunk {
            if !banned_roots.contains(&rq.root) {
                for kind in rq.kinds.iter() {
                    let x = format!("kind({}, {})", kind, rq.root);
                    if buffer.is_empty() {
                        buffer.write_all(x.as_bytes()).unwrap();
                    } else {
                        buffer.write_all(union_with_spaces_bytes).unwrap();
                        buffer.write_all(x.as_bytes()).unwrap();
                    }
                }
            }
        }
        String::from_utf8(buffer).unwrap()
    };
    let res = bazel_query
        .execute(&[
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
                .or_insert_with(HashSet::default);
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
            .find(|e| e.starts_with("workspace("));
        if let Some(line) = ln {
            if let Some(captures) = RE.captures(line) {
                return Some(String::from(captures.get(2).unwrap().as_str()));
            }
        }
    }
    None
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::parse();

    let parsed_command_line = match parse_bazel_command_line(&[opt.bazel_binary_path.to_string_lossy().to_string()], Default::default()) {
        Ok(parsed_command_line) => {
            if parsed_command_line.is_action_option_set("bes_backend") {
                eprintln!("Bes backend already set, must exit since we can't add another safely.");
                std::process::exit(-1);
            }
            parsed_command_line
        }
        Err(cmd_line_parsing_failed) => {
            match cmd_line_parsing_failed {
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::MissingBazelPath => {
                    eprintln!("Missing bazel path, invalid command line arg supplied");
                    std::process::exit(-1);
                }
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::MissingArgToOption(o) => {
                    eprintln!("Arg parsing from bazelfe doesn't understand the args, missing an option to {}", o);
                    std::process::exit(-1);
                }
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::UnknownArgument(o) => {
                    eprintln!("Arg parsing from bazelfe doesn't understand the args, unknown option {}", o);
                    std::process::exit(-1);
                }
            }
        }
    };

    let config = load_config_file(&opt.config.as_ref()).await?;

    let _rng = rand::thread_rng();
    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    let running_refresh_mode = false;
    builder.target(pretty_env_logger::env_logger::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("warn,bazelfe_core::jvm_indexer=info,jvm_indexer=info");
    }
    builder.init();

    let target_blacklist = opt.blacklist_targets_from_index.clone().unwrap_or_default();

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

    let _union_with_spaces_bytes = " union ".as_bytes();

    let all_targets_to_use = {
        info!("Executing initial query to find all external repos in this bazel repository");

        let res = bazel_query
            .execute(&[String::from("query"), String::from("//external:*")])
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
        blacklist_repos.extend(
            opt.blacklist_remote_roots
                .iter()
                .flat_map(|e| e.split(','))
                .map(|e| e.to_string()),
        );

        info!(
            "Will ignore any target in the repos: {}",
            blacklist_repos.join(",")
        );

        for line in res.stdout.lines() {
            if let Some(ln) = line.strip_prefix("//external:") {
                let mut ok = true;
                'inner: for root in blacklist_repos.iter() {
                    if ln.starts_with(root) {
                        ok = false;
                        break 'inner;
                    }
                }
                // Some externals are bind mounts
                if ln.contains('/') {
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

        let query_rule_attr_batch_size: usize = 40;
        info!("Extracting targets with an allowed rule kind, gives rise to {} total queries, we will union them to bazel in batches of size: {}", all_queries.len(), query_rule_attr_batch_size);

        let mut all_targets_to_use: HashMap<String, HashSet<String>> = HashMap::default();
        let mut processed_count = 0;
        let mut remaining_chunks: Vec<Vec<RuleQuery>> = all_queries
            .chunks(query_rule_attr_batch_size)
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
            if !current_chunk.is_empty() {
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
                            if let Some(captures) = NO_REPO_REGEX.captures(ln) {
                                let repo = captures.get(1).unwrap().as_str().to_string();
                                info!("Ignoring non existant repo: {}", repo);
                                global_banned_roots.insert(repo);
                                matched = true;
                            }

                            if let Some(captures) = OTHER_NO_REPO_REGEX.captures(ln) {
                                let repo = captures.get(1).unwrap().as_str().to_string();
                                info!("Ignoring non existant repo: {}", repo);
                                global_banned_roots.insert(repo);
                                matched = true;
                            }

                            if let Some(captures) = NO_SUCH_FILE.captures(ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }
                            if let Some(captures) = NOT_RESOLVABLE.captures(ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }

                            if let Some(captures) = NO_TARGETS_FOUND.captures(ln) {
                                must_go_solo.insert(captures.get(1).unwrap().as_str().to_string());
                                matched = true;
                            }

                            if !matched && ln.starts_with("ERROR:") {
                                have_unmatched_line = true;
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

                            remaining_chunks.extend(current_chunk.chunks(next_chunk_len).map(
                                |e| {
                                    e.iter()
                                        .cloned()
                                        .filter(|e| {
                                            !must_go_solo.contains(&e.root)
                                                && !global_banned_roots.contains(&e.root)
                                        })
                                        .collect()
                                },
                            ));
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
        let space_section = " ".repeat(spaces);
        info!("{}{}{}", k, space_section, v.len());
        for e in v.iter() {
            all_found_targets.insert(e.clone());
        }
    }

    let index_table = if running_refresh_mode && opt.index_output_location.exists() {
        let mut src_f = std::fs::File::open(&opt.index_output_location).unwrap();
        bazelfe_core::index_table::IndexTable::read(&mut src_f).unwrap_or_default()
    } else {
        bazelfe_core::index_table::IndexTable::default()
    };

    for e in target_blacklist {
        index_table.add_target_to_blacklist(e).await
    }

    let target_completed_tracker = TargetCompletedTracker::new(all_found_targets);

    let popularity_data =
        bazelfe_core::jvm_indexer::popularity_parser::build_popularity_map().await;

    for (k, v) in popularity_data {
        index_table.set_popularity_str(k, v as u16).await
    }

    let addr: Option<std::net::SocketAddr> = opt
        .bind_address
        .or_else(|| env::var("BIND_ADDRESS").ok())
        .map(|e| e.parse().expect("can't parse BIND_ADDRESS variable"));

    let bazel_wrapper_builder = BazelWrapperBuilder {
        bes_server_bind_address: addr,
        processors: vec![
            Arc::new(IndexNewResults::new(
                index_table.clone(),
                &config.indexer_config,
            )),
            Arc::new(target_completed_tracker.clone()),
        ],
    };

    let bazel_wrapper = bazel_wrapper_builder.build().await?;

    let compile_batch_size: usize = 1000;
    info!(
        "About to start building targets, will occur in batches of size: {}",
        compile_batch_size
    );

    async fn run_bazel(
        bazel_wrapper: &BazelWrapper<BuildEventResponse>,
        parsed_command_line: &ParsedCommandLine,
        batch_idx: usize,
        chunk: &mut Vec<String>,
        target_completed_tracker: &TargetCompletedTracker,
    ) {
        let batch_idx = batch_idx;
        let batch_start_time = Instant::now();

        let mut parsed_command_line = parsed_command_line.clone();
        parsed_command_line.set_action(Some(bazel_command_line_parser::Action::BuiltIn(
            bazel_command_line_parser::BuiltInAction::Build,
        )));

        parsed_command_line.add_action_option_if_unset(
            bazel_command_line_parser::BazelOption::BooleanOption(String::from("keep_going"), true),
        );

        parsed_command_line.remaining_args.append(chunk);

        let (tx, rx) = async_channel::unbounded();
        let recv_task = tokio::spawn(async move { while rx.recv().await.is_ok() {} });

        let bazel_result = bazel_wrapper
            .spawn_bazel_attempt(&parsed_command_line, false, tx)
            .await
            .expect("Should succeed");
        recv_task.await.unwrap();
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
        .flat_map(|(_, e)| e.into_iter())
    {
        if batch_elements.len() >= compile_batch_size {
            run_bazel(
                &bazel_wrapper,
                &parsed_command_line,
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
        &bazel_wrapper,
        &parsed_command_line,
        batch_idx,
        &mut batch_elements,
        &target_completed_tracker,
    )
    .await;

    info!("Building a target popularity map");

    info!("Writing out index data");

    let mut file = std::fs::File::create(&opt.index_output_location).unwrap();

    index_table.write(&mut file).await;

    Ok(())
}
