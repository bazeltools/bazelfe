use std::{path::PathBuf, time::Instant};
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
    pub shared_last_files: Arc<SharedLastFiles>
}

#[tarpc::server]
impl super::daemon_service::RunnerDaemon for DaemonServerInstance {
    async fn recently_changed_files(
        self,
        _: tarpc::context::Context,
    ) -> Vec<super::daemon_service::FileStatus> {
        todo!()
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
    last_files_updated: Arc<Mutex<Vec<(PathBuf, Instant)>>>
}
impl SharedLastFiles {
    pub fn new() -> Self {
        Self {
            last_files_updated: Arc::new(Mutex::new(Vec::default()))
        }
    }
    pub async fn register_new_file(&self, path: PathBuf) -> () {
        let mut lock = self.last_files_updated.lock().await;
        lock.push((path, Instant::now()));
    }

    pub async fn get_recent_files(&self) -> Vec<(PathBuf, usize)> {
        let lock = self.last_recent_files.lock().await;
        // lock.iter().map(|(k, t)|{
        //     t.duration
        // })
        vec![]
    }

}

pub async fn main(
    daemon_config: &DaemonConfig,
    _bazel_binary_path: &PathBuf,
    paths: &super::daemon_manager::DaemonPaths,
) -> Result<(), Box<dyn Error>> {
    super::setup_daemon_io(&daemon_config.daemon_communication_folder)?;

    let shared_last_files = Arc::new(SharedLastFiles::new());
    let current_dir = std::env::current_dir().expect("Failed to determine current directory");

    let current_dir = current_dir;

    use notify::{RecommendedWatcher, RecursiveMode, Watcher};

    let mut watcher: RecommendedWatcher = Watcher::new_immediate(|res| match res {
        Ok(event) => println!("event: {:?}", event),
        Err(e) => println!("watch error: {:?}", e),
    })
    .unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(current_dir, RecursiveMode::Recursive)
        .unwrap();

        let captured = Arc::clone(&shared_last_files);
    start_tarpc_server(&paths.socket_path, move || {

        DaemonServerInstance {
            shared_last_files: captured.clone()
        }
    }).await?;

    eprintln!("Daemon process is up! and serving on socket");
    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

    eprintln!("Daemon terminating after 200 seconds.");

    Ok(())
}
