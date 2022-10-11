use bazelfe_protos::bazel_tools::daemon_service::daemon_service_server::DaemonService;
use bazelfe_protos::*;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{
    collections::{HashMap, HashSet},
    ops::{Add, Sub},
    path::PathBuf,
    sync::atomic::{AtomicU32, AtomicUsize},
    time::Duration,
};
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response};

use bazel_tools::daemon_service;
use dashmap::DashMap;
use std::error::Error;
use tokio::{sync::Mutex, task::JoinHandle};

use crate::config::daemon_config::NotifyRegexes;
use crate::config::DaemonConfig;
use std::time::Instant;
use tokio::net::UnixListener;

#[derive(Debug, Clone)]
struct Daemon {
    _config: Arc<DaemonConfig>,
    _bazel_binary_path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Distance(pub u16);

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum TargetType {
    Rule(RuleTarget),
    Src(SrcFileTarget),
}
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct RuleTarget {
    pub target_label: String,
    pub target_kind: String,
    pub is_test: bool,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SrcFileTarget {
    pub target_label: String,
}

#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone)]
pub struct TargetId(u32);

#[derive(Debug)]
struct TargetState {
    src_file_to_target: DashMap<PathBuf, TargetId>,
    target_to_rdeps: DashMap<TargetId, HashSet<TargetId>>,
    target_id_to_details: DashMap<TargetId, TargetType>,
    label_string_to_id: DashMap<String, TargetId>,
    max_target_id: AtomicU32,
}
impl Default for TargetState {
    fn default() -> Self {
        Self {
            src_file_to_target: Default::default(),
            target_to_rdeps: Default::default(),
            target_id_to_details: Default::default(),
            label_string_to_id: Default::default(),
            max_target_id: AtomicU32::new(0),
        }
    }
}

// If data arrives too quickly, we may start reporting times in the future!
static mut CURRENT_TIME: u64 = 0;
fn monotonic_current_time() -> daemon_service::Instant {
    let ret = unsafe { CURRENT_TIME };
    unsafe {
        CURRENT_TIME += 1;
    }
    daemon_service::Instant { value: ret }
}
fn target_as_path(s: &str) -> Option<PathBuf> {
    let pb = PathBuf::from(s.replace(':', "/").replace("//", ""));
    if pb.exists() {
        Some(pb)
    } else {
        None
    }
}
impl TargetState {
    async fn ingest_new_deps(&self, dependencies_calculated: &blaze_query::QueryResult) {
        for target in dependencies_calculated.target.iter() {
            if let Some(rule) = target.rule.as_ref() {
                if !self.label_string_to_id.contains_key(&rule.name) {
                    let cur_id = TargetId(self.max_target_id.fetch_add(1, Ordering::AcqRel));

                    let target_data = RuleTarget {
                        target_label: rule.name.clone(),
                        target_kind: rule.rule_class.clone(),
                        is_test: rule.rule_class.ends_with("_test"),
                    };
                    self.target_id_to_details
                        .insert(cur_id, TargetType::Rule(target_data));

                    self.label_string_to_id.insert(rule.name.clone(), cur_id);

                    for rdep in rule.rule_output.iter() {
                        self.label_string_to_id.insert(rdep.clone(), cur_id);
                    }
                } else {
                    eprintln!("Skipping {}", rule.name);
                }
            }

            if let Some(src_file) = target.source_file.as_ref() {
                eprintln!("Looking at src file {:#?}", src_file);
                if let Some(path) = target_as_path(&src_file.name) {
                    let cur_id = TargetId(self.max_target_id.fetch_add(1, Ordering::AcqRel));
                    eprintln!("Inserting known file {:#?}", path);
                    self.src_file_to_target.insert(path, cur_id);
                    self.label_string_to_id
                        .insert(src_file.name.clone(), cur_id);

                    let srcfile_data = SrcFileTarget {
                        target_label: src_file.name.clone(),
                    };
                    self.target_id_to_details
                        .insert(cur_id, TargetType::Src(srcfile_data));
                }
            }
        }

        for target in dependencies_calculated.target.iter() {
            if let Some(rule) = target.rule.as_ref() {
                let rdep_src: TargetId = *self
                    .label_string_to_id
                    .get(&rule.name)
                    .expect("Expected to find target")
                    .value();

                for rdep in rule.rule_input.iter() {
                    if let Some(id) = self.label_string_to_id.get(rdep) {
                        let id: TargetId = *id.value();
                        if !self.target_to_rdeps.contains_key(&id) {
                            self.target_to_rdeps.insert(id, Default::default());
                        }
                        let mut t = self
                            .target_to_rdeps
                            .get_mut(&id)
                            .expect("We guaranteed its here.");

                        t.insert(rdep_src);
                    } else {
                        eprintln!("For rule {}, skipping input: {}", rule.name, rdep);
                    }
                }
            }
        }
    }
    pub async fn hydrate_new_file_data(
        self: Arc<TargetState>,
        bazel_query: Arc<Mutex<Box<dyn BazelQuery>>>,
        path: &Path,
    ) -> Result<(), Box<dyn Error>> {
        if self.src_file_to_target.contains_key(path) {
            return Ok(());
        }
        let mut cur_path = Some(path);
        while let Some(p) = cur_path {
            if p.join("BUILD").exists() || p.join("WORKSPACE").exists() {
                break;
            } else {
                cur_path = p.parent();
            }
        }

        if let Some(p) = cur_path {
            let bazel_query = bazel_query.lock().await;
            if self.src_file_to_target.contains_key(path) {
                return Ok(());
            }
            eprintln!(
                "Missing data for {:#?}, going to run query operations",
                path
            );
            let dependencies_calculated = crate::bazel_query::graph_query(
                bazel_query.as_ref(),
                &format!("deps({}:all, 1)", p.to_string_lossy()),
                &[],
                true,
            )
            .await?;

            self.ingest_new_deps(&dependencies_calculated).await;

            for target in dependencies_calculated.target.iter() {
                if let Some(rule) = &target.rule {
                    let rdep_src: TargetId = *self
                        .label_string_to_id
                        .get(&rule.name)
                        .expect("Expected to find target")
                        .value();

                    let need_query: bool = self
                        .target_to_rdeps
                        .get(&rdep_src)
                        .map(|e| e.value().is_empty())
                        .unwrap_or(true);
                    if need_query {
                        let dependencies_calculated = crate::bazel_query::graph_query(
                            bazel_query.as_ref(),
                            &format!("rdeps(//..., {})", &rule.name),
                            &[],
                            true,
                        )
                        .await?;

                        self.ingest_new_deps(&dependencies_calculated).await;
                    } else {
                        eprintln!("{} doesn't need query", rule.name);
                    }
                }
            }
        }

        Ok(())
    }
}
use crate::jvm_indexer::bazel_query::BazelQuery;

#[derive(Debug)]
struct UpdatedFileState {
    content_sha: Option<Vec<u8>>,
    daemon_updated: daemon_service::Instant,
    updated: Instant,
}

#[derive(Debug)]
struct TargetCache {
    target_state: Arc<TargetState>,
    last_files_updated: Arc<Mutex<HashMap<PathBuf, UpdatedFileState>>>,
    inotify_ignore_regexes: NotifyRegexes,
    pending_hydrations: Arc<AtomicUsize>,
    bazel_query: Arc<Mutex<Box<dyn BazelQuery>>>,
    inotify_receiver: Arc<flume::Receiver<daemon_service::Instant>>,
    inotify_sender: Arc<flume::Sender<daemon_service::Instant>>,
    last_update_ts: Arc<Mutex<daemon_service::Instant>>,
}

impl TargetCache {
    pub fn new(
        daemon_config: &DaemonConfig,
        bazel_query: &Arc<Mutex<Box<dyn BazelQuery>>>,
    ) -> Self {
        let (inotify_event_occured, inotify_receiver) =
            flume::unbounded::<daemon_service::Instant>();

        Self {
            target_state: Default::default(),
            last_files_updated: Default::default(),
            inotify_ignore_regexes: daemon_config.inotify_ignore_regexes.clone(),
            pending_hydrations: Arc::new(AtomicUsize::new(0)),
            bazel_query: bazel_query.clone(),
            inotify_receiver: Arc::new(inotify_receiver),
            inotify_sender: Arc::new(inotify_event_occured),
            last_update_ts: Arc::new(Mutex::new(monotonic_current_time())),
        }
    }

    async fn hydrate_new_file_data(&self, path: PathBuf) {
        self.pending_hydrations.fetch_add(1, Ordering::Release);

        let pending_hydrations = self.pending_hydrations.clone();
        let target_state = self.target_state.clone();
        let bazel_query = self.bazel_query.clone();
        tokio::task::spawn(async move {
            if let Err(e) = target_state.hydrate_new_file_data(bazel_query, &path).await {
                eprintln!(
                    "Failed to hydrate {}, error:\n{:#?}",
                    path.to_string_lossy(),
                    e
                );
            }
            pending_hydrations.fetch_sub(1, Ordering::Release);
        });
    }

    pub async fn register_new_files(&self, paths: Vec<PathBuf>, event_kind: notify::EventKind) {
        let current_path = std::env::current_dir().expect("Should be able to get the current dir");
        let mut lock = self.last_files_updated.lock().await;
        let ts = monotonic_current_time();
        let now_instant = Instant::now();
        for p in paths.clone() {
            let file_name = if let Some(file_name) = p.file_name() {
                file_name.to_os_string()
            } else {
                continue;
            };

            let parent = if let Some(file_name) = p.parent() {
                file_name.to_path_buf()
            } else {
                continue;
            };

            let parent_relative = if let Ok(relative_path) = parent
                .canonicalize()
                .unwrap_or_else(|_| p.clone())
                .strip_prefix(current_path.as_path())
            {
                relative_path.to_path_buf()
            } else {
                continue;
            };

            let real_path = parent_relative.join(file_name);

            let is_ignored = self
                .inotify_ignore_regexes
                .0
                .iter()
                .find(|&p| p.is_match(real_path.to_string_lossy().as_ref()));
            if is_ignored.is_some() {
                continue;
            }

            let real_metadata = if let Ok(m) = std::fs::symlink_metadata(&real_path) {
                m
            } else {
                continue;
            };

            let src_metadata = if let Ok(m) = std::fs::symlink_metadata(&p) {
                m
            } else {
                continue;
            };

            // Modifying a directory isn't interesting.
            if event_kind.is_modify() && real_metadata.is_dir() {
                continue;
            }
            // Other events to folders is super noisy.
            if event_kind.is_other() && real_metadata.is_dir() {
                continue;
            }
            if real_path.exists() && real_metadata.file_type() == src_metadata.file_type() {
                let mut current_sha = None;
                let mut do_insert = true;
                if real_metadata.is_file() {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    if let Ok(mut file) = std::fs::File::open(&real_path) {
                        if std::io::copy(&mut file, &mut hasher).is_ok() {
                            current_sha = Some(hasher.finalize().to_vec());

                            if let Some(p) = lock.get(&real_path) {
                                if let Some(prev_sha) = p.content_sha.as_ref() {
                                    if current_sha.as_ref() == Some(prev_sha) {
                                        do_insert = false;
                                    }
                                }
                            }
                        }
                    }
                }

                if do_insert {
                    self.hydrate_new_file_data(real_path.clone()).await;
                    eprintln!(
                        "Noting changed file, Event kind: {:#?} at a given timestamp. {:#?}",
                        event_kind,
                        (real_path.to_path_buf(), (ts, now_instant, &current_sha))
                    );
                    lock.insert(
                        real_path.to_path_buf(),
                        UpdatedFileState {
                            content_sha: current_sha,
                            daemon_updated: ts,
                            updated: now_instant,
                        },
                    );
                }
            }
        }
        *self.last_update_ts.lock().await = ts;
        let _ = self.inotify_sender.send(ts);

        let mut max_age = Duration::from_secs(3600);

        while lock.len() > 20 && max_age > Duration::from_secs(120) {
            lock.retain(|_, v| now_instant - v.updated < max_age);
            max_age /= 2;
        }
    }

    pub async fn wait_for_files(
        &self,
        instant: daemon_service::Instant,
    ) -> Vec<daemon_service::FileStatus> {
        let start_time = Instant::now();
        let max_wait = Duration::from_millis(20);
        let spin_wait = Duration::from_millis(3);

        loop {
            if *self.last_update_ts.lock().await > instant {
                debug!("Last update timing indicates we should just try get recent files as of instant: {}", instant.value);
                return self.get_recent_files(instant).await;
            }

            if Instant::now().sub(start_time) > max_wait {
                debug!("Timing out and returning empty set of mutated files.");
                return Vec::default();
            }
            if let Ok(v) = self
                .inotify_receiver
                .recv_deadline(start_time.add(spin_wait))
            {
                if v > instant {
                    debug!("Returning recent files since update timestamp is good");
                    return self.get_recent_files(instant).await;
                }
            }
        }
    }

    pub async fn get_recent_files(
        &self,
        instant: daemon_service::Instant,
    ) -> Vec<daemon_service::FileStatus> {
        let lock = self.last_files_updated.lock().await;

        lock.iter()
            .filter_map(|(k, prev_s)| {
                let v = prev_s.daemon_updated;
                if v > instant {
                    Some(daemon_service::FileStatus {
                        path: k.to_string_lossy().to_string(),
                        updated: Some(v),
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct DaemonServerInstance {
    pub executable_id: Arc<super::ExecutableId>,
    pub most_recent_call: Arc<AtomicUsize>,
    pub target_cache: Arc<TargetCache>,
    pub _daemon_config: Arc<DaemonConfig>,
    pub _bazel_binary_path: Arc<PathBuf>,
}

#[tonic::async_trait]
impl DaemonService for DaemonServerInstance {
    async fn wait_for_files(
        &self,
        request: Request<daemon_service::WaitForFilesRequest>,
    ) -> Result<Response<daemon_service::WaitForFilesResponse>, tonic::Status> {
        let request = request.into_inner();
        let since_when = request
            .value
            .unwrap_or(daemon_service::Instant { value: 0 });

        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        let files = self.target_cache.wait_for_files(since_when).await;

        Ok(Response::new(daemon_service::WaitForFilesResponse {
            value: files,
        }))
    }

    async fn recently_changed_files(
        &self,
        request: Request<daemon_service::RecentlyChangedFilesRequest>,
    ) -> Result<Response<daemon_service::RecentlyChangedFilesResponse>, tonic::Status> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        let instant = request.into_inner().value.unwrap_or_default();
        let files = self.target_cache.get_recent_files(instant).await;
        Ok(Response::new(
            daemon_service::RecentlyChangedFilesResponse { value: files },
        ))
    }

    async fn ping(
        &self,
        _: Request<daemon_service::PingRequest>,
    ) -> Result<Response<daemon_service::PingResponse>, tonic::Status> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        Ok(Response::new(daemon_service::PingResponse {
            executable_id: Some(self.executable_id.as_ref().clone()),
        }))
    }

    async fn recently_invalidated_targets(
        &self,
        request: Request<daemon_service::RecentlyInvalidatedTargetsRequest>,
    ) -> Result<Response<daemon_service::RecentlyInvalidatedTargetsResponse>, tonic::Status> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        let request = request.into_inner();

        let recent_files = self
            .target_cache
            .get_recent_files(daemon_service::Instant { value: 0 })
            .await;
        let targets = match self
            .targets_from_files(Request::new(daemon_service::TargetsFromFilesRequest {
                files: recent_files,
                distance: request.distance,
                was_in_query: true,
            }))
            .await?
            .into_inner()
            .response
            .unwrap_or(daemon_service::targets_from_files_response::Response::InQuery(true))
        {
            daemon_service::targets_from_files_response::Response::InQuery(_) => Vec::default(),
            daemon_service::targets_from_files_response::Response::Targets(t) => t.targets,
        };

        Ok(Response::new(
            daemon_service::RecentlyInvalidatedTargetsResponse {
                targets: Some(daemon_service::Targets { targets }),
            },
        ))
    }

    async fn targets_from_files(
        &self,
        request: Request<daemon_service::TargetsFromFilesRequest>,
    ) -> Result<Response<daemon_service::TargetsFromFilesResponse>, tonic::Status> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        let request = request.into_inner();
        let was_in_query = request.was_in_query;
        let distance = request.distance;
        let files = request.files;

        let start_time = Instant::now();
        while self
            .target_cache
            .pending_hydrations
            .load(std::sync::atomic::Ordering::Acquire)
            > 0
        {
            if start_time.elapsed() > Duration::from_millis(100) || !was_in_query {
                return Ok(Response::new(daemon_service::TargetsFromFilesResponse {
                    response: Some(
                        daemon_service::targets_from_files_response::Response::InQuery(true),
                    ),
                }));
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        let target_ids = files.iter().filter_map(|f| {
            let path = &f.path;
            self.target_cache
                .target_state
                .src_file_to_target
                .get(&PathBuf::from(path))
                .map(|e| *e.value())
        });

        let mut active_targets_ids: HashSet<TargetId> = target_ids.collect();

        for _ in 0..distance {
            let mut next_targets: HashSet<TargetId> = HashSet::default();

            for e in active_targets_ids.iter() {
                if let Some(rdeps) = self.target_cache.target_state.target_to_rdeps.get(e) {
                    for rdep in rdeps.value() {
                        next_targets.insert(*rdep);
                    }
                }
            }
            active_targets_ids = next_targets;
        }

        let mut result_targets = Vec::default();

        for rt in active_targets_ids.into_iter() {
            let target_data = self
                .target_cache
                .target_state
                .target_id_to_details
                .get(&rt)
                .unwrap();
            match target_data.value() {
                TargetType::Rule(r) => {
                    if r.is_test {
                        result_targets.push(daemon_service::Target {
                            target_response: Some(
                                daemon_service::target::TargetResponse::TestLabel(
                                    r.target_label.clone(),
                                ),
                            ),
                        });
                    } else {
                        result_targets.push(daemon_service::Target {
                            target_response: Some(
                                daemon_service::target::TargetResponse::BuildLabel(
                                    r.target_label.clone(),
                                ),
                            ),
                        });
                    }
                }
                TargetType::Src(_) => {}
            }
        }

        return Ok(Response::new(daemon_service::TargetsFromFilesResponse {
            response: Some(
                daemon_service::targets_from_files_response::Response::Targets(
                    daemon_service::Targets {
                        targets: result_targets,
                    },
                ),
            ),
        }));
    }

    async fn request_instant(
        &self,
        _: Request<daemon_service::RequestInstantRequest>,
    ) -> Result<Response<daemon_service::RequestInstantResponse>, tonic::Status> {
        Ok(Response::new(daemon_service::RequestInstantResponse {
            value: Some(monotonic_current_time()),
        }))
    }
}

async fn start_server(
    path: &PathBuf,
    daemon_server_builder: DaemonServerInstance,
) -> Result<JoinHandle<()>, Box<dyn Error>> {
    let bind_path = PathBuf::from(path);
    let uds = UnixListener::bind(bind_path)?;
    let uds_stream = UnixListenerStream::new(uds);

    Ok(tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                daemon_service::daemon_service_server::DaemonServiceServer::new(
                    daemon_server_builder,
                ),
            )
            .serve_with_incoming(uds_stream)
            .await
            .unwrap()
    }))
}

pub async fn main_from_config(config_path: &PathBuf) -> Result<(), Box<dyn Error>> {
    use std::fs::File;
    use std::io::BufReader;

    // Open the file in read-only mode with buffer.
    let file = File::open(config_path)?;
    let reader = BufReader::new(file);

    // Read the JSON contents of the file as an instance of `User`.
    let u: super::daemon_manager::HydratedDaemonConfig = serde_json::from_reader(reader)?;

    main(&u.daemon_config, &u.bazel_binary_path, &u.daemon_paths).await
}

use clap::Parser;
#[derive(Parser, Debug)]
#[clap(name = "basic")]
struct DaemonArgs {
    pub config_path: PathBuf,
}

pub async fn base_main() -> Result<(), Box<dyn Error>> {
    let opt = DaemonArgs::parse();
    if let Err(e) = main_from_config(&opt.config_path).await {
        eprintln!("Daemon for bazel runner failed with error: {:#}", e);
        println!("Daemon for bazel runner failed with error: {:#}", e);
    }

    Ok(())
}

pub async fn main(
    daemon_config: &DaemonConfig,
    bazel_binary_path: &Path,
    paths: &super::DaemonPaths,
) -> Result<(), Box<dyn Error>> {
    super::setup_daemon_io(&daemon_config.daemon_communication_folder)?;

    let bazel_query: Arc<Mutex<Box<dyn BazelQuery>>> = Arc::new(Mutex::new(Box::new(
        crate::jvm_indexer::bazel_query::from_binary_path(bazel_binary_path),
    )));

    println!("Starting up bazelfe daemon");
    let executable_id = Arc::new(super::current_executable_id());

    let target_cache = Arc::new(TargetCache::new(daemon_config, &bazel_query));
    let current_dir = std::env::current_dir().expect("Failed to determine current directory");

    let most_recent_call = Arc::new(AtomicUsize::new(0));

    let captured_most_recent_call = most_recent_call.clone();

    let captured_target_cache = target_cache.clone();

    let captured_daemon_config = Arc::new(daemon_config.clone());
    let captured_bazel_binary_path = Arc::new(bazel_binary_path.to_path_buf());
    println!("Starting Grpc service");

    start_server(
        &paths.socket_path,
        DaemonServerInstance {
            executable_id: executable_id.clone(),
            most_recent_call: captured_most_recent_call.clone(),
            target_cache: captured_target_cache.clone(),
            _daemon_config: captured_daemon_config.clone(),
            _bazel_binary_path: captured_bazel_binary_path.clone(),
        },
    )
    .await?;

    let current_dir = current_dir;

    let mut ignore_builder = ignore::gitignore::GitignoreBuilder::new(&current_dir);

    for f in &[".gitignore", ".bazelignore"] {
        let sub_path = current_dir.join(f);
        if sub_path.exists() {
            let _ = ignore_builder.add(sub_path);
        }
    }
    let gitignore_match = Arc::new(
        ignore_builder
            .build()
            .expect("Should be able to build from gitignore"),
    );

    use notify::{RecommendedWatcher, RecursiveMode, Watcher};

    let (flume_tx, flume_rx) = flume::unbounded::<notify::Event>();
    let copy_shared = Arc::clone(&target_cache);
    let copy_gitignored = Arc::clone(&gitignore_match);

    println!("Starting inotify watchers");
    let _ = tokio::task::spawn(async move {
        while let Ok(event) = flume_rx.recv_async().await {
            use notify::EventKind;

            let should_process = match &event.kind {
                EventKind::Any => true,
                EventKind::Access(access_type) => matches!(
                    access_type,
                    notify::event::AccessKind::Close(notify::event::AccessMode::Write)
                ),
                EventKind::Create(_) => true,
                EventKind::Modify(_) => true,
                EventKind::Remove(_) => true,
                EventKind::Other => true,
            };

            let filtered_paths: Vec<PathBuf> = event
                .paths
                .iter()
                .filter(|path| !copy_gitignored.matched(path, path.is_dir()).is_ignore())
                .cloned()
                .collect();

            if should_process && !filtered_paths.is_empty() {
                copy_shared
                    .register_new_files(filtered_paths, event.kind.clone())
                    .await;
            }
        }
    });

    println!("Starting inotify watcher");
    let mut core_watcher: RecommendedWatcher =
        RecommendedWatcher::new(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                if let Err(e) = flume_tx.send(event) {
                    eprintln!("Failed to enqueue inotify event: {:#?}", e);
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })
        .unwrap();

    core_watcher
        .configure(notify::Config::PreciseEvents(true))
        .unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    for entry in std::fs::read_dir(&current_dir)? {
        let entry = entry?;
        let path = entry.path();

        let is_ignored = daemon_config
            .inotify_ignore_regexes
            .0
            .iter()
            .find(|&p| p.is_match(entry.file_name().to_string_lossy().as_ref()));
        if is_ignored.is_some() {
            continue;
        }

        if gitignore_match.matched(&path, path.is_dir()).is_ignore() {
            continue;
        }
        eprintln!("Watching {:#?}", path);

        core_watcher.watch(&path, RecursiveMode::Recursive).unwrap();
    }

    let core_watcher = Arc::new(std::sync::Mutex::new(core_watcher));

    let notify_ignore_regexes = daemon_config.inotify_ignore_regexes.clone();

    let mut root_watcher: RecommendedWatcher =
        RecommendedWatcher::new(move |res: notify::Result<notify::Event>| match res {
            Ok(event) => {
                if let notify::EventKind::Create(_) = event.kind {
                    for path in event.paths.iter() {
                        let file_name = if let Some(file_name) = path.file_name() {
                            file_name
                        } else {
                            continue;
                        };
                        let is_ignored = notify_ignore_regexes
                            .0
                            .iter()
                            .find(|&p| p.is_match(file_name.to_string_lossy().as_ref()));
                        if is_ignored.is_some() {
                            continue;
                        }

                        if gitignore_match.matched(path, path.is_dir()).is_ignore() {
                            continue;
                        }

                        let mut core_watcher = core_watcher.lock().unwrap();
                        eprintln!("Watching {:#?}", path);

                        core_watcher.watch(path, RecursiveMode::Recursive).unwrap();
                    }
                }
            }
            Err(e) => println!("watch error: {:?}", e),
        })
        .unwrap();

    root_watcher
        .watch(&current_dir, RecursiveMode::NonRecursive)
        .unwrap();

    eprintln!("Daemon process is up! and serving on socket");
    let mut last_call = usize::MAX;
    let mut last_seen = Instant::now();

    let max_delay = Duration::from_secs(3600);

    println!("Looping to track activity.");
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        let current_v = most_recent_call.load(std::sync::atomic::Ordering::Acquire);

        if current_v == last_call {
            // If we haven't incremented since the last loop
            // and we haven't incremented in max_delay time then exit
            let now = Instant::now();
            let elapsed = now.duration_since(last_seen);
            if elapsed > max_delay {
                eprintln!(
                    "Quitting since its been {:#?} which is more than {:#?}",
                    elapsed, max_delay
                );
                break;
            }
        } else {
            last_call = current_v;
            last_seen = Instant::now();
        }
        let pid = super::read_pid(paths);
        if let Some(p) = pid {
            // Another process lauched and we didn't catch the conflict in the manager, we should die to avoid issues.
            let our_pid = std::process::id();
            if our_pid != p as u32 {
                eprintln!(
                    "Quitting since our pid is {}, but pid file contains {}",
                    our_pid, p
                );
                break;
            }
        } else {
            eprintln!("Quitting since cannot open pid file");
            break; // directory or file gone. Die.
        }
    }

    Ok(())
}
