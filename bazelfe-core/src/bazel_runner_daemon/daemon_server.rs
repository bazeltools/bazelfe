use std::path::PathBuf;
use std::time::SystemTime;

use std::{error::Error, sync::Arc};
use tarpc::serde_transport as transport;
use tarpc::server::Channel;
use tokio::{sync::Mutex, task::JoinHandle};

use crate::bazel_runner_daemon::daemon_service::RunnerDaemon;
use crate::config::DaemonConfig;
use tokio::net::UnixListener;
use tokio_serde::formats::Bincode;
use tokio_util::codec::LengthDelimitedCodec;

#[derive(Debug, Clone)]
struct Daemon {
    config: Arc<DaemonConfig>,
    bazel_binary_path: PathBuf,
}

#[derive(Debug, Clone)]
struct DaemonServerInstance {
    pub shared_last_files: Arc<SharedLastFiles>,
}

#[tarpc::server]
impl super::daemon_service::RunnerDaemon for DaemonServerInstance {
    async fn recently_changed_files(
        self,
        _: tarpc::context::Context,
    ) -> Vec<super::daemon_service::FileStatus> {
        self.shared_last_files.get_recent_files().await
    }

    async fn ping(self, _: tarpc::context::Context) -> () {
        ()
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
    last_files_updated: Arc<Mutex<Vec<(PathBuf, SystemTime)>>>,
}
impl SharedLastFiles {
    pub fn new() -> Self {
        Self {
            last_files_updated: Arc::new(Mutex::new(Vec::default())),
        }
    }
    pub async fn register_new_files(&self, paths: Vec<PathBuf>) -> () {
        let mut lock = self.last_files_updated.lock().await;
        let t = SystemTime::now();
        for p in paths {
            lock.push((p, t));
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
        // lock.iter().map(|(k, t)|{
        //     t.duration
        // })
    }
}
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
    _bazel_binary_path: &PathBuf,
    paths: &super::daemon_manager::DaemonPaths,
) -> Result<(), Box<dyn Error>> {
    super::setup_daemon_io(&daemon_config.daemon_communication_folder)?;

    eprintln!("Here!!!");
    let shared_last_files = Arc::new(SharedLastFiles::new());
    let current_dir = std::env::current_dir().expect("Failed to determine current directory");

    let captured = Arc::clone(&shared_last_files);
    start_tarpc_server(&paths.socket_path, move || DaemonServerInstance {
        shared_last_files: captured.clone(),
    })
    .await?;

    let current_dir = current_dir;
    eprintln!("Here!!!");

    use notify::{RecommendedWatcher, RecursiveMode, Watcher};

    let (flume_tx, flume_rx) = flume::unbounded::<notify::Event>();
    let copy_shared = Arc::clone(&shared_last_files);

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

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(current_dir, RecursiveMode::Recursive)
        .unwrap();

    eprintln!("Daemon process is up! and serving on socket");
    tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;

    eprintln!("Daemon terminating after 60 seconds.");

    Ok(())
}
