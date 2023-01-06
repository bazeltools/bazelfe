use log::debug;
use log::info;
use ptyprocess::PtyProcess;
use std::ffi::OsString;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

// 0 == no sub process
// -1 == don't send signals
// > 0 == send signals
static SUB_PROCESS_PID: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static CTRL_C_HANLDER_SET: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

mod bazel_wrapper;
mod bazel_wrapper_builder;
mod bazel_wrapper_error;
pub use bazel_wrapper::BazelWrapper;
pub use bazel_wrapper_builder::BazelWrapperBuilder;
pub use bazel_wrapper_error::BazelWrapperError;

pub mod user_report_error;
use crate::bazel_command_line_parser::ParsedCommandLine;
pub use user_report_error::UserReportError;

pub fn register_ctrlc_handler() {
    if CTRL_C_HANLDER_SET
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        // OK this is already set.
        return;
    }
    ctrlc::set_handler(move || {
        let current_sub_process_pid: i32 = SUB_PROCESS_PID.load(Ordering::SeqCst);

        let state = if current_sub_process_pid == 0 {
            "Subprocess active"
        } else {
            "no subprocess active"
        };
        info!("Received ctrl-c, state: {:?}", state);

        // no subprocess_pid
        if current_sub_process_pid == 0 {
            info!(
                "Shutting down via ctrl-c, sub_process_pid: {:?}",
                current_sub_process_pid
            );
            std::process::exit(137);
        } else if current_sub_process_pid > 0 {
            // To ensure we killl any spawned of or exec'd sub processes get the grp and send the signal to everyone.
            let child_pid = nix::unistd::Pid::from_raw(current_sub_process_pid);
            if let Ok(grp) = nix::unistd::getpgid(Some(child_pid)) {
                debug!("Sending kill signal to {:#?}", grp);
                nix::sys::signal::killpg(grp, nix::sys::signal::Signal::SIGINT).unwrap();
            }
            debug!("Sending kill signal to {:#?}", &child_pid);
            nix::sys::signal::kill(child_pid, nix::sys::signal::Signal::SIGINT).unwrap();
        }
    })
    .expect("Error setting Ctrl-C handler");
}

fn add_custom_args(bazel_command_line: &mut ParsedCommandLine, srv_port: u16) {
    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::OptionWithArg(
            String::from("bes_timeout"),
            String::from("300000ms"),
        ),
    );

    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::BooleanOption(
            String::from("legacy_important_outputs"),
            false,
        ),
    );

    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::OptionWithArg(
            String::from("experimental_build_event_upload_strategy"),
            String::from("local"),
        ),
    );

    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::BooleanOption(
            String::from("build_event_text_file_path_conversion"),
            true,
        ),
    );

    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::OptionWithArg(
            String::from("color"),
            String::from("yes"),
        ),
    );

    bazel_command_line.add_action_option_if_unset(
        crate::bazel_command_line_parser::BazelOption::OptionWithArg(
            String::from("bes_backend"),
            format!("grpc://127.0.0.1:{}", srv_port),
        ),
    );
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecuteResult {
    pub exit_code: i32,
}
pub async fn execute_bazel(
    bazel_command_line: &ParsedCommandLine,
    bes_port: u16,
) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
    execute_bazel_output_control(bazel_command_line, bes_port, true).await
}

// use tokio when we aren't dealing with a tty.
async fn execute_tokio_subprocess(
    command: &Path,
    args: &Vec<OsString>,
    show_output: bool,
) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
    use tokio::process::Command;

    let mut cmd = Command::new(command);

    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child: tokio::process::Child = cmd.spawn().expect("failed to start bazel process");
    SUB_PROCESS_PID.store(-1, Ordering::SeqCst);

    let mut child_stdout = child.stdout.take().expect("Child didn't have a stdout");

    let stdout = tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut stdout = tokio::io::stdout();

        while let Ok(bytes_read) = child_stdout.read(&mut buffer[..]).await {
            if bytes_read == 0 {
                break;
            }
            if show_output {
                if let Err(_) = stdout.write_all(&buffer[0..bytes_read]).await {
                    break;
                }
            }
        }
    });

    let mut child_stderr = child.stderr.take().expect("Child didn't have a stderr");

    let stderr = tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut stderr = tokio::io::stderr();
        while let Ok(bytes_read) = child_stderr.read(&mut buffer[..]).await {
            if bytes_read == 0 {
                break;
            }
            if show_output {
                if let Err(_) = stderr.write_all(&buffer[0..bytes_read]).await {
                    break;
                }
            }
        }
    });
    let result = child.wait().await.expect("The command wasn't running");

    // These tasks can/will fail when a chained process or otherwise can close the input/output pipe.
    // e.g. bazel help test | head -n 5
    // would cause stdout to fail here.
    let _ = stderr.await;
    let _ = stdout.await;

    SUB_PROCESS_PID.store(0, Ordering::SeqCst);

    Ok(ExecuteResult {
        exit_code: result.code().unwrap_or(-1),
    })
}

async fn execute_sub_tty_process(
    command: &Path,
    args: &Vec<OsString>,
) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
    use std::process::Command;
    let mut cmd = Command::new(command);
    cmd.args(args);

    let child: PtyProcess = PtyProcess::spawn(cmd).expect("failed to start bazel process");

    SUB_PROCESS_PID.store(child.pid().as_raw(), Ordering::SeqCst);

    let mut child_fd = child.get_raw_handle().unwrap();

    let stderr = tokio::task::spawn_blocking(move || {
        let mut buffer = [0; 2048];
        let mut stderr = std::io::stderr();

        while let Ok(bytes_read) = child_fd.read(&mut buffer[..]) {
            if bytes_read == 0 {
                break;
            }
            if stderr.write_all(&buffer[0..bytes_read]).is_err() {
                break;
            }
            if stderr.flush().is_err() {
                break;
            }
        }
    });

    let child_complete: ptyprocess::WaitStatus = tokio::task::spawn_blocking(move || child.wait())
        .await
        .expect("The command wasn't running")?;

    SUB_PROCESS_PID.store(0, Ordering::SeqCst);

    // These tasks can/will fail when a chained process or otherwise can close the input/output pipe.
    // e.g. bazel help test | head -n 5
    // would cause stdout to fail here.
    let _ = stderr.await;

    let exit_code = if let ptyprocess::WaitStatus::Exited(_pid, code) = child_complete {
        code
    } else {
        -1
    };
    Ok(ExecuteResult { exit_code })
}

pub async fn execute_bazel_output_control(
    bazel_command_line: &ParsedCommandLine,
    bes_port: u16,
    show_output: bool,
) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
    let mut bazel_command_line = bazel_command_line.clone();

    add_custom_args(&mut bazel_command_line, bes_port);

    debug!("{:#?}", bazel_command_line);

    let args: Vec<OsString> = bazel_command_line
        .all_args_normalized()?
        .into_iter()
        .map(|e| e.into())
        .collect();

    use crossterm::tty::IsTty;
    use std::io::stdout;

    if show_output && stdout().is_tty() {
        execute_sub_tty_process(&bazel_command_line.bazel_binary, &args).await
    } else {
        execute_tokio_subprocess(&bazel_command_line.bazel_binary, &args, show_output).await
    }
}
