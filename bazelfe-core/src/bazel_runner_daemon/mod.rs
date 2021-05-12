use std::path::PathBuf;

pub mod query_graph;

pub mod daemon_manager;
pub mod daemon_server;

use fork::Fork;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum SpawnFailure {
    #[error("Error spawning child from command line process: `{0}`")]
    PrimaryForkFailure(i32),

    #[error("In child process unable to close stdin: `{0}`")]
    UnableToCloseStdin(i32),

    #[error("Inner fork from child to fork grandchild failed: `{0}`")]
    ForkToGranChildFailed(i32),

    #[error("Create session and set process group ID for daemon failed: `{0}`")]
    SetSidFailure(i32),

    #[error("Unable to make directories")]
    MakeDirFailed(std::io::Error),

    #[error("Unable to make directories")]
    TouchLogFile(std::io::Error),

    #[error("Failed to do generic io operation")]
    IoError(#[from] std::io::Error),

    #[error("Failed to exec in child daemon process")]
    ExecvpError(#[from] exec::Error),
}

const OUTPUT_SUFFIXES: [&str; 2] = ["stdout", "stderr"];

fn make_paths<'a>(root: &'a PathBuf) -> impl Iterator<Item = PathBuf> + 'a {
    OUTPUT_SUFFIXES
        .iter()
        .map(move |suffix| root.join(format!("{}.log", suffix)))
}

fn setup_daemon_io(root: &PathBuf) -> Result<(), SpawnFailure> {
    std::fs::create_dir_all(&root).map_err(|e| SpawnFailure::MakeDirFailed(e))?;
    for path in make_paths(&root) {
        std::fs::File::create(path).map_err(|e| SpawnFailure::TouchLogFile(e))?;
    }

    use stdio_override::*;
    let guard = StdoutOverride::override_file(root.join(format!("{}.log", "stdout")))?;
    std::mem::forget(guard);
    let guard = StderrOverride::override_file(root.join(format!("{}.log", "stderr")))?;
    std::mem::forget(guard);

    Ok(())
}

fn close_stdin() -> Result<(), SpawnFailure> {
    match unsafe { libc::close(0) } {
        -1 => Err(SpawnFailure::UnableToCloseStdin(-1)),
        _ => Ok(()),
    }
}

pub fn spawn_daemon<S>(pid_path: &PathBuf, child_process_args: &[S]) -> Result<(), SpawnFailure>
where
    S: AsRef<std::ffi::OsStr>,
{
    if let Some(parent) = pid_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SpawnFailure::MakeDirFailed(e))?;
    }

    match fork::fork() {
        Ok(Fork::Parent(_)) => Ok(()),
        Ok(Fork::Child) => fork::setsid()
            .map_err(|e| SpawnFailure::SetSidFailure(e))
            .and_then(|_| {
                close_stdin()?;
                match fork::fork() {
                    Ok(Fork::Parent(_)) => std::process::exit(0),
                    Ok(Fork::Child) => {
                        use std::io::Write;
                        let mut file = std::fs::File::create(pid_path)?;
                        file.write_all(format!("{}", std::process::id()).as_bytes())?;
                        drop(file);

                        if let Ok(root_path) = std::env::var("REPO_ROOT") {
                            std::env::set_current_dir(root_path)?;
                        }
                        std::env::set_var("BAZEL_FE_ENABLE_DAEMON_MODE", "true");
                        let e = exec::Command::new(std::env::current_exe()?)
                            .args(child_process_args)
                            .exec();
                        Err(e)?
                    }
                    Err(e) => Err(SpawnFailure::ForkToGranChildFailed(e)),
                }
            }),
        Err(n) => Err(SpawnFailure::PrimaryForkFailure(n)),
    }
}

pub mod daemon_service {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    #[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
    pub struct FileStatus(pub PathBuf, pub u128);
    #[tarpc::service]
    pub trait RunnerDaemon {
        async fn recently_changed_files() -> Vec<FileStatus>;

        async fn ping() -> ();
    }
}
