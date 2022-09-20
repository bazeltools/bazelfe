use std::path::{Path, PathBuf};

pub mod daemon_manager;
pub mod daemon_server;

use bazelfe_protos::bazel_tools::daemon_service::ExecutableId;
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

    #[error("Unable to find the WORKSPACE file, our current working directory was calculated out to be: `{0}`, try setting the REPO_ROOT env var")]
    UnableToFindWorkspace(PathBuf),

    #[error("Failed to exec in child daemon process")]
    ExecvpError(#[from] exec::Error),
}

const OUTPUT_SUFFIXES: [&str; 2] = ["stdout", "stderr"];

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct DaemonPaths {
    pub logs_path: PathBuf,
    pub pid_path: PathBuf,
    pub socket_path: PathBuf,
}

fn make_paths(root: &'_ Path) -> impl Iterator<Item = PathBuf> + '_ {
    OUTPUT_SUFFIXES
        .iter()
        .map(move |suffix| root.join(format!("{}.log", suffix)))
}

fn setup_daemon_io(root: &Path) -> Result<(), SpawnFailure> {
    std::fs::create_dir_all(&root).map_err(SpawnFailure::MakeDirFailed)?;
    for path in make_paths(root) {
        std::fs::File::create(path).map_err(SpawnFailure::TouchLogFile)?;
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
        std::fs::create_dir_all(parent).map_err(SpawnFailure::MakeDirFailed)?;
    }

    let mut path_to_use =
        std::env::current_dir().expect("Should be able to determine the current dir");
    if let Ok(root_path) = std::env::var("REPO_ROOT") {
        path_to_use = PathBuf::from(root_path);
    }

    if !std::env::current_dir()?.join("WORKSPACE").exists() {
        return Err(SpawnFailure::UnableToFindWorkspace(path_to_use));
    }

    match fork::fork() {
        Ok(Fork::Parent(_)) => Ok(()),
        Ok(Fork::Child) => fork::setsid()
            .map_err(SpawnFailure::SetSidFailure)
            .and_then(|_| {
                close_stdin()?;
                match fork::fork() {
                    Ok(Fork::Parent(_)) => std::process::exit(0),
                    Ok(Fork::Child) => {
                        use std::io::Write;
                        let mut file = std::fs::File::create(pid_path)?;
                        file.write_all(format!("{}", std::process::id()).as_bytes())?;
                        drop(file);

                        std::env::set_current_dir(path_to_use)?;
                        std::env::set_var("BAZEL_FE_ENABLE_DAEMON_MODE", "true");
                        let e = exec::Command::new(std::env::current_exe()?)
                            .args(child_process_args)
                            .exec();
                        Err(e.into())
                    }
                    Err(e) => Err(SpawnFailure::ForkToGranChildFailed(e)),
                }
            }),
        Err(n) => Err(SpawnFailure::PrimaryForkFailure(n)),
    }
}

pub fn current_executable_id() -> ExecutableId {
    ExecutableId {
        build_timestamp: String::from(env!("VERGEN_BUILD_TIMESTAMP")),
        git_branch: String::from(env!("VERGEN_GIT_BRANCH")),
        git_sha: String::from(env!("VERGEN_GIT_SHA")),
    }
}

pub(in crate::bazel_runner_daemon) fn read_pid(daemon_paths: &DaemonPaths) -> Option<i32> {
    use std::fs::File;
    use std::io::prelude::*;

    if let Ok(mut file) = File::open(&daemon_paths.pid_path) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            if let Ok(parsed) = contents.parse() {
                return Some(parsed);
            }
        }
    }
    None
}
