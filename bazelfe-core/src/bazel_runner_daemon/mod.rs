use std::path::PathBuf;

use fork::Fork;
use thiserror::Error;
pub enum DaemonType {
    CommandLineCli,
    DaemonProcess,
}

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
}

const OUTPUT_SUFFIXES: [&str; 2] = ["stdout", "stderr"];

fn make_paths<'a>(root: &'a PathBuf) -> impl Iterator<Item = PathBuf> + 'a {
    OUTPUT_SUFFIXES
        .iter()
        .map(move |suffix| root.join(format!("{}.log", suffix)))
}

fn setup_daemon_io(root: &PathBuf) -> Result<(), SpawnFailure> {
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

pub fn spawn_daemon(communication_path: PathBuf) -> Result<DaemonType, SpawnFailure> {
    std::fs::create_dir_all(&communication_path).map_err(|e| SpawnFailure::MakeDirFailed(e))?;

    for path in make_paths(&communication_path) {
        std::fs::File::create(path).map_err(|e| SpawnFailure::TouchLogFile(e))?;
    }

    match fork::fork() {
        Ok(Fork::Parent(_)) => Ok(DaemonType::CommandLineCli),
        Ok(Fork::Child) => fork::setsid()
            .map_err(|e| SpawnFailure::SetSidFailure(e))
            .and_then(|_| {
                close_stdin()?;
                setup_daemon_io(&communication_path)?;
                match fork::fork() {
                    Ok(Fork::Parent(_)) => std::process::exit(0),
                    Ok(Fork::Child) => {
                        use std::io::Write;
                        let mut file =
                            std::fs::File::create(communication_path.join("daemon.pid"))?;
                        file.write_all(format!("{}", std::process::id()).as_bytes())?;
                        drop(file);
                        Ok(DaemonType::DaemonProcess)
                    }
                    Err(e) => Err(SpawnFailure::ForkToGranChildFailed(e)),
                }
            }),
        Err(n) => Err(SpawnFailure::PrimaryForkFailure(n)),
    }
}
