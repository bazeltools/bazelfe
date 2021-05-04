use std::error::Error;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    match bazelfe_core::bazel_runner_daemon::spawn_daemon(PathBuf::from("/tmp/daemon_talk"))? {
        bazelfe_core::bazel_runner_daemon::DaemonType::CommandLineCli => {
            eprintln!("Command line client wing...{}", std::process::id());
        }
        bazelfe_core::bazel_runner_daemon::DaemonType::DaemonProcess => {
            std::thread::sleep(std::time::Duration::from_secs(2));
            eprintln!("DaemonProcess wing...{}", std::process::id());
        }
    }
    Ok(())
}
