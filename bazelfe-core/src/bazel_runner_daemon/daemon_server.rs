use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicU32, AtomicUsize},
    time::Duration,
};
use std::{sync::atomic::Ordering, time::SystemTime};

use dashmap::DashMap;
use std::{error::Error, sync::Arc};
use tarpc::serde_transport as transport;
use tarpc::server::Channel;
use tokio::{sync::Mutex, task::JoinHandle};

use crate::config::DaemonConfig;
use crate::{
    bazel_runner_daemon::daemon_service::RunnerDaemon, config::daemon_config::NotifyRegexes,
};
use std::time::Instant;
use tokio::net::UnixListener;
use tokio_serde::formats::Bincode;
use tokio_util::codec::LengthDelimitedCodec;

#[derive(Debug, Clone)]
struct Daemon {
    config: Arc<DaemonConfig>,
    bazel_binary_path: PathBuf,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Distance(pub u16);

#[derive(Debug, PartialEq, Eq)]
pub struct TargetData {
    pub target_label: String,
    pub target_kind: String,
    pub is_test: bool,
}

#[derive(Debug, Copy, PartialEq, Eq, Hash, Clone)]
pub struct TargetId(u32);

#[derive(Debug)]
struct TargetState {
    src_file_to_target: DashMap<PathBuf, TargetId>,
    target_to_deps: DashMap<TargetId, Vec<TargetId>>,
    target_id_to_details: DashMap<TargetId, TargetData>,
    label_string_to_id: DashMap<String, TargetId>,
    max_target_id: AtomicU32,
}
impl Default for TargetState {
    fn default() -> Self {
        Self {
            src_file_to_target: Default::default(),
            target_to_deps: Default::default(),
            target_id_to_details: Default::default(),
            label_string_to_id: Default::default(),
            max_target_id: AtomicU32::new(0),
        }
    }
}

fn target_as_path(s: &String) -> Option<PathBuf> {
    let pb = PathBuf::from(s.replace(":", "/").replace("//", ""));
    if pb.exists() {
        Some(pb)
    } else {
        None
    }
}
impl TargetState {
    pub async fn hydrate_new_file_data(
        self: Arc<TargetState>,
        bazel_query: Arc<dyn BazelQuery>,
        path: &PathBuf,
    ) -> Result<(), Box<dyn Error>> {
        if self.src_file_to_target.contains_key(path) {
            return Ok(());
        }

        eprintln!(
            "Checking for path: {:#?}, found... : {:#?}",
            path, self.label_string_to_id
        );

        let mut cur_path = Some(path.as_path());
        loop {
            if let Some(p) = cur_path {
                if p.join("BUILD").exists() || p.join("WORKSPACE").exists() {
                    break;
                } else {
                    cur_path = p.parent();
                }
            } else {
                break;
            };
        }
        if let Some(p) = cur_path {
            let dependencies_calculated = crate::bazel_runner_daemon::query_graph::graph_query(
                bazel_query.as_ref(),
                &format!("deps({}, 1)", p.to_string_lossy()),
            )
            .await;

            for (k, rdeps) in dependencies_calculated.iter() {
                if !self.label_string_to_id.contains_key(k) {
                    let cur_id = TargetId(self.max_target_id.fetch_add(1, Ordering::AcqRel));
                    self.label_string_to_id
                        .insert(path.to_string_lossy().to_string(), cur_id);
                    if let Some(path) = target_as_path(k) {
                        self.label_string_to_id
                            .insert(path.to_string_lossy().to_string(), cur_id);
                    }
                }
            }
            eprintln!("Dependencies... {:#?}", dependencies_calculated);
        }

        Ok(())
    }
}
use crate::jvm_indexer::bazel_query::BazelQuery;
#[derive(Debug)]
struct TargetCache {
    target_state: Arc<TargetState>,
    last_files_updated: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
    inotify_ignore_regexes: NotifyRegexes,
    pending_hydrations: Arc<AtomicUsize>,
    bazel_query: Arc<dyn BazelQuery>,
}

impl TargetCache {
    pub fn new(daemon_config: &DaemonConfig, bazel_query: &Arc<dyn BazelQuery>) -> Self {
        Self {
            target_state: Default::default(),
            last_files_updated: Default::default(),
            inotify_ignore_regexes: daemon_config.inotify_ignore_regexes.clone(),
            pending_hydrations: Arc::new(AtomicUsize::new(0)),
            bazel_query: bazel_query.clone(),
        }
    }

    async fn hydrate_new_file_data(&self, path: PathBuf) -> () {
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
    pub async fn register_new_files(&self, paths: Vec<PathBuf>) -> () {
        let current_path = std::env::current_dir().expect("Should be able to get the current dir");
        let mut lock = self.last_files_updated.lock().await;
        let t = SystemTime::now();
        for p in paths {
            if let Ok(relative_path) = p
                .canonicalize()
                .unwrap_or(p)
                .strip_prefix(current_path.as_path())
            {
                let is_ignored = self
                    .inotify_ignore_regexes
                    .0
                    .iter()
                    .find(|&p| p.is_match(relative_path.to_string_lossy().as_ref()));
                if is_ignored.is_none() {
                    let pb = relative_path.to_path_buf();
                    self.hydrate_new_file_data(pb.clone()).await;
                    lock.insert(relative_path.to_path_buf(), t);
                }
            }
        }
        let mut max_age = Duration::from_secs(3600);
        while lock.len() > 20 && max_age > Duration::from_secs(120) {
            lock.retain(|_, v| t.duration_since(*v).unwrap() < max_age);
            max_age /= 2;
        }
    }

    pub async fn get_recent_files(&self) -> Vec<super::daemon_service::FileStatus> {
        let lock = self.last_files_updated.lock().await;

        lock.iter()
            .map(|(k, v)| {
                super::daemon_service::FileStatus(
                    k.clone(),
                    v.duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u128,
                )
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
struct DaemonServerInstance {
    pub executable_id: Arc<super::ExecutableId>,
    pub most_recent_call: Arc<AtomicUsize>,
    pub target_cache: Arc<TargetCache>,
    pub daemon_config: Arc<DaemonConfig>,
    pub bazel_binary_path: Arc<PathBuf>,
}

#[tarpc::server]
impl super::daemon_service::RunnerDaemon for DaemonServerInstance {
    async fn recently_changed_files(
        self,
        _: tarpc::context::Context,
    ) -> Vec<super::daemon_service::FileStatus> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.target_cache.get_recent_files().await
    }

    async fn ping(self, _: tarpc::context::Context) -> super::ExecutableId {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);
        self.executable_id.as_ref().clone()
    }

    async fn recently_invalidated_targets(
        self,
        _: tarpc::context::Context,
        distance: u32,
    ) -> Vec<super::daemon_service::Targets> {
        self.most_recent_call
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        let recent_files = self.target_cache.get_recent_files().await;

        // let mut target_cache = self.target_cache.target_state.lock().await;

        // let paths_missing_owner = recent_files.iter().filter(|&f| {
        //     let path = &f.0;
        //     !target_cache.src_file_to_target.contains_key(path)
        // });

        // let bazel_query =
        //     crate::jvm_indexer::bazel_query::from_binary_path(self.bazel_binary_path.as_ref());

        // for path in paths_missing_owner {
        //     let mut cur_path = Some(path.0.as_path());
        //     loop {
        //         if let Some(p) = cur_path {
        //             if p.join("BUILD").exists() || p.join("WORKSPACE").exists() {
        //                 break;
        //             } else {
        //                 cur_path = p.parent();
        //             }
        //         } else {
        //             break;
        //         };
        //     }
        //     if let Some(p) = cur_path {
        //         let dependencies_calculated = crate::bazel_runner_daemon::query_graph::graph_query(
        //             &bazel_query,
        //             &format!("deps({}, 1)", p.to_string_lossy()),
        //         )
        //         .await;
        //         eprintln!("{:#?}", dependencies_calculated);
        //     }
        // }

        Vec::default()
    }
}

async fn start_tarpc_server<F>(
    path: &PathBuf,
    daemon_server_builder: F,
) -> Result<JoinHandle<()>, Box<dyn Error>>
where
    F: Fn() -> DaemonServerInstance + Send + 'static,
{
    let bind_path = PathBuf::from(path);
    let stream = UnixListener::bind(bind_path)?;
    let codec_builder = LengthDelimitedCodec::builder();

    Ok(tokio::spawn(async move {
        loop {
            if let Ok((conn, _)) = stream.accept().await {
                let framed = codec_builder.new_framed(conn);
                let transport = transport::new(framed, Bincode::default());

                eprintln!("Client connected!");
                let fut = tarpc::server::BaseChannel::with_defaults(transport)
                    .execute(daemon_server_builder().serve());

                tokio::spawn(async move {
                    fut.await;
                });
            } else {
                eprintln!("Socket dead, quitting.");

                break;
            }
        }
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

#[derive(Debug, Clone)]
struct SharedLastFiles {
    last_files_updated: Arc<Mutex<HashMap<PathBuf, SystemTime>>>,
    inotify_ignore_regexes: NotifyRegexes,
}
impl SharedLastFiles {}
use clap::Clap;
#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct DaemonArgs {
    #[clap(parse(from_os_str))]
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
    bazel_binary_path: &PathBuf,
    paths: &super::DaemonPaths,
) -> Result<(), Box<dyn Error>> {
    super::setup_daemon_io(&daemon_config.daemon_communication_folder)?;

    let bazel_query: Arc<dyn BazelQuery> = Arc::new(
        crate::jvm_indexer::bazel_query::from_binary_path(bazel_binary_path),
    );

    println!("Starting up bazelfe daemon");
    let executable_id = Arc::new(super::current_executable_id());
    let target_cache = Arc::new(TargetCache::new(daemon_config, &bazel_query));
    let current_dir = std::env::current_dir().expect("Failed to determine current directory");

    let most_recent_call = Arc::new(AtomicUsize::new(0));

    let captured_most_recent_call = most_recent_call.clone();

    let captured_target_cache = target_cache.clone();

    let captured_daemon_config = Arc::new(daemon_config.clone());
    let captured_bazel_binary_path = Arc::new(bazel_binary_path.clone());
    println!("Starting tarpc");
    start_tarpc_server(&paths.socket_path, move || DaemonServerInstance {
        executable_id: executable_id.clone(),
        most_recent_call: captured_most_recent_call.clone(),
        target_cache: captured_target_cache.clone(),
        daemon_config: captured_daemon_config.clone(),
        bazel_binary_path: captured_bazel_binary_path.clone(),
    })
    .await?;

    let current_dir = current_dir;

    use notify::{RecommendedWatcher, RecursiveMode, Watcher};

    let (flume_tx, flume_rx) = flume::unbounded::<notify::Event>();
    let copy_shared = Arc::clone(&target_cache);

    println!("Starting tarpc");
    let _ = tokio::task::spawn(async move {
        while let Ok(event) = flume_rx.recv_async().await {
            use notify::EventKind;
            let should_process = match &event.kind {
                EventKind::Any => true,
                EventKind::Access(_) => false,
                EventKind::Create(_) => true,
                EventKind::Modify(_) => true,
                EventKind::Remove(_) => true,
                EventKind::Other => true,
            };

            if should_process {
                copy_shared.register_new_files(event.paths).await;
            }
        }
    });

    println!("Starting inotify watcher");
    let mut watcher: RecommendedWatcher =
        Watcher::new_immediate(move |res: notify::Result<notify::Event>| {
            match res {
                Ok(event) => {
                    if let Err(e) = flume_tx.send(event) {
                        eprintln!("Failed to enqueue inotify event: {:#?}", e);
                    }
                }
                Err(e) => println!("watch error: {:?}", e),
            }
            ()
        })
        .unwrap();

    watcher
        .configure(notify::Config::NoticeEvents(true))
        .unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    eprintln!("Watching {:#?}", current_dir);
    watcher
        .watch(current_dir, RecursiveMode::Recursive)
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
            if now.duration_since(last_seen) > max_delay {
                eprintln!("Quitting since its now {:#?} and we haven't seen an update since {:#?} which is more than {:#?}", now, last_seen, max_delay);
                break;
            }
        } else {
            last_call = current_v;
            last_seen = Instant::now();
        }
        let pid = super::read_pid(&paths);
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
