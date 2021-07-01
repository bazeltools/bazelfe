use std::error::Error;
use std::path::PathBuf;

use crate::config::DaemonConfig;
use serde::{Deserialize, Serialize};

use super::DaemonPaths;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HydratedDaemonConfig {
    pub daemon_config: DaemonConfig,
    pub bazel_binary_path: PathBuf,
    pub daemon_paths: DaemonPaths,
}
async fn start_server(
    daemon_config: &DaemonConfig,
    bazel_binary_path: &PathBuf,
    paths: &DaemonPaths,
) -> Result<(), Box<dyn Error>> {
    let _ = std::fs::remove_file(&paths.pid_path);
    let _ = std::fs::remove_file(&paths.socket_path);

    let merged_config = HydratedDaemonConfig {
        daemon_config: daemon_config.clone(),
        bazel_binary_path: bazel_binary_path.clone(),
        daemon_paths: paths.clone(),
    };

    let child_cfg = daemon_config
        .daemon_communication_folder
        .join("config.json");

    let file = std::fs::File::create(&child_cfg)?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::ser::to_writer_pretty(&mut writer, &merged_config)?;
    drop(writer);

    let o: std::ffi::OsString = child_cfg.to_string_lossy().to_string().into();
    crate::bazel_runner_daemon::spawn_daemon(&paths.pid_path, &[&o])?;

    Ok(())
}

async fn try_kill_server(paths: &DaemonPaths) -> () {
    if let Some(pid) = super::read_pid(&paths) {
        signal_mgr::kill(pid)
    }
}

mod signal_mgr {
    pub fn process_is_alive(pid: i32) -> bool {
        unsafe { libc::kill(pid, 0) == 0 }
    }

    pub fn kill(pid: i32) -> () {
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

async fn maybe_connect_to_server(
    paths: &DaemonPaths,
    executable_id: &super::ExecutableId,
) -> Result<Option<super::daemon_service::RunnerDaemonClient>, Box<dyn Error>> {
    if !paths.pid_path.exists() {
        return Ok(None);
    }

    if let Some(pid) = super::read_pid(&paths) {
        if !signal_mgr::process_is_alive(pid) {
            return Ok(None);
        }
    } else {
        return Ok(None);
    }

    for _ in 0..10 {
        if !paths.socket_path.exists() {
            tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
        }
    }

    if !paths.socket_path.exists() {
        try_kill_server(&paths).await;
        return Ok(None);
    }

    use tokio::net::UnixStream;
    use tokio_serde::formats::Bincode;
    use tokio_util::codec::LengthDelimitedCodec;
    let codec_builder = LengthDelimitedCodec::builder();

    let conn = UnixStream::connect(&paths.socket_path).await?;

    let transport = tarpc::serde_transport::new(codec_builder.new_framed(conn), Bincode::default());
    if let Ok(cli) =
        super::daemon_service::RunnerDaemonClient::new(Default::default(), transport).spawn()
    {
        match cli.ping(tarpc::context::current()).await {
            Ok(remote_id) => {
                if executable_id == &remote_id {
                    return Ok(Some(cli));
                } else {
                    try_kill_server(&paths).await;
                    return Ok(None);
                }
            }
            Err(err) => {
                eprintln!(
                    "Connected to daemon process, but ping failed with error: {:#?}",
                    err
                );
                return Ok(None);
            }
        }
    }
    Ok(None)
}

pub async fn connect_to_server(
    daemon_config: &DaemonConfig,
    bazel_binary_path: &PathBuf,
) -> Result<Option<super::daemon_service::RunnerDaemonClient>, Box<dyn Error>> {
    std::fs::create_dir_all(&daemon_config.daemon_communication_folder)?;

    if !daemon_config.enabled {
        return Ok(None);
    }

    let executable_id = super::current_executable_id();

    let paths = DaemonPaths {
        logs_path: daemon_config.daemon_communication_folder.clone(),
        pid_path: daemon_config
            .daemon_communication_folder
            .clone()
            .join("server.pid"),
        socket_path: daemon_config
            .daemon_communication_folder
            .clone()
            .join("server.sock"),
    };

    let mut cntr = 0;
    while cntr < 3 {
        cntr += 1;

        let connection = maybe_connect_to_server(&paths, &executable_id).await?;

        if connection.is_some() {
            return Ok(connection);
        }

        if cntr < 3 {
            start_server(daemon_config, bazel_binary_path, &paths).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(4)).await;
        }
    }

    return Ok(None);
}
