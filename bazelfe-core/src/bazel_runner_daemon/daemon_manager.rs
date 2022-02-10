use std::path::PathBuf;
use std::{error::Error, path::Path};

use crate::config::DaemonConfig;
use anyhow::{anyhow, Context};
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

pub(in crate) async fn try_kill_server_from_cfg(daemon_config: &DaemonConfig) {
    if let Ok(daemon_communication_ptr) = configure_communication_ptr(&daemon_config) {
        let paths = daemon_paths_from_access(&daemon_communication_ptr);

        if let Some(pid) = super::read_pid(&paths) {
            signal_mgr::kill(pid)
        }
    }
}

async fn try_kill_server(paths: &DaemonPaths) {
    if let Some(pid) = super::read_pid(paths) {
        signal_mgr::kill(pid)
    }
}

mod signal_mgr {
    pub fn process_is_alive(pid: i32) -> bool {
        unsafe { libc::kill(pid, 0) == 0 }
    }

    pub fn kill(pid: i32) {
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

    if let Some(pid) = super::read_pid(paths) {
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
        try_kill_server(paths).await;
        return Ok(None);
    }

    use tokio::net::UnixStream;
    use tokio_serde::formats::Bincode;
    use tokio_util::codec::LengthDelimitedCodec;
    let codec_builder = LengthDelimitedCodec::builder();

    let conn = UnixStream::connect(&paths.socket_path).await?;

    let transport = tarpc::serde_transport::new(codec_builder.new_framed(conn), Bincode::default());
    let cli = super::daemon_service::RunnerDaemonClient::new(Default::default(), transport).spawn();

    match cli.ping(tarpc::context::current()).await {
        Ok(remote_id) => {
            if executable_id == &remote_id {
                Ok(Some(cli))
            } else {
                try_kill_server(paths).await;
                Ok(None)
            }
        }
        Err(err) => {
            eprintln!(
                "Connected to daemon process, but ping failed with error: {:#?}",
                err
            );
            Ok(None)
        }
    }
}

fn daemon_paths_from_access(access_path: &Path) -> DaemonPaths {
    DaemonPaths {
        logs_path: access_path.to_path_buf(),
        pid_path: access_path.to_path_buf().join("server.pid"),
        socket_path: access_path.to_path_buf().join("server.sock"),
    }
}

fn configure_communication_ptr(daemon_config: &DaemonConfig) -> Result<PathBuf, Box<dyn Error>> {
    std::fs::create_dir_all(&daemon_config.daemon_communication_folder)?;

    let current_dir = std::env::current_dir().expect("Should be able to get the current dir");
    if !current_dir.join("WORKSPACE").exists() {
        return Err(anyhow!(
            "Expected the CWD to be a root of a bazel repo, but unable to find a WORKSPACE file"
        )
        .into());
    }

    let bazelfe_path = current_dir.join("bazel-bazelfe");

    if bazelfe_path.exists() {
        let metadata = std::fs::symlink_metadata(&bazelfe_path).with_context(|| {
            "Expect to be able to try get metadata for the bazel-bazelfe even if it doesn't exist"
        })?;

        if !metadata.is_symlink() {
            return Err(anyhow!("Expected bazel-bazelfe to be a symlink, but it wasn't..").into());
        }

        let target = std::fs::read_link(&bazelfe_path)?;

        if target != daemon_config.daemon_communication_folder {
            return Err(anyhow!("Exepected bazel-bazelfe to point at the expected communication daemon folder. {}, but it pointed at {}. If in doubt, remove this symlink.", daemon_config.daemon_communication_folder.to_string_lossy(), target.to_string_lossy()).into());
        }
    } else {
        std::os::unix::fs::symlink(&daemon_config.daemon_communication_folder, &bazelfe_path).with_context(|| "Expected to be able to build a symlink from the CWD to the bazelfe communication folder")?;
    }
    Ok(bazelfe_path)
}
pub async fn connect_to_server(
    daemon_config: &DaemonConfig,
    bazel_binary_path: &Path,
) -> Result<Option<super::daemon_service::RunnerDaemonClient>, Box<dyn Error>> {
    if !daemon_config.enabled {
        debug!("Daemon isn't requested/needed. Noop.");
        return Ok(None);
    }
    let executable_id = super::current_executable_id();

    let daemon_communication_ptr = configure_communication_ptr(&daemon_config)?;
    let paths = daemon_paths_from_access(&daemon_communication_ptr);

    let mut cntr = 0;
    while cntr < 3 {
        cntr += 1;

        let connection = maybe_connect_to_server(&paths, &executable_id).await?;

        if connection.is_some() {
            return Ok(connection);
        }

        if cntr < 3 {
            start_server(daemon_config, &bazel_binary_path.to_path_buf(), &paths).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(4)).await;
        }
    }

    Ok(None)
}
