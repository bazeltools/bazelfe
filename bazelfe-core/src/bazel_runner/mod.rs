use std::ffi::OsString;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use tokio::process::Command;

static SUB_PROCESS_PID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

mod auto_test_action;
pub mod bazel_runner;
mod command_line_rewriter_action;
mod configured_bazel_runner;
mod processor_activity;
mod user_report_error;
pub use user_report_error::UserReportError;

use crate::bazel_command_line_parser::ParsedCommandLine;

pub fn register_ctrlc_handler() {
    ctrlc::set_handler(move || {
        let current_sub_process_pid: u32 = SUB_PROCESS_PID.load(Ordering::SeqCst);
        info!("Received ctrl-c, state: {:?}", current_sub_process_pid);

        // no subprocess_pid
        if current_sub_process_pid == 0 {
            info!(
                "Shutting down via ctrl-c, sub_process_pid: {:?}",
                current_sub_process_pid
            );
            std::process::exit(137);
        }
    })
    .expect("Error setting Ctrl-C handler");
}

fn add_custom_args(bazel_command_line: &mut ParsedCommandLine, srv_port: u16) -> () {
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

#[derive(Clone, PartialEq, Debug)]
pub struct ExecuteResult {
    pub exit_code: i32,
    pub errors_corrected: u32,
}
pub async fn execute_bazel(
    bazel_command_line: &ParsedCommandLine,
    bes_port: u16,
) -> Result<ExecuteResult, Box<dyn std::error::Error>> {
    execute_bazel_output_control(bazel_command_line, bes_port, true).await
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

    let mut cmd = Command::new(bazel_command_line.bazel_binary);

    cmd.args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child: tokio::process::Child = cmd.spawn().expect("failed to start bazel process");
    SUB_PROCESS_PID.store(child.id().unwrap(), Ordering::SeqCst);

    let mut child_stdout = child.stdout.take().expect("Child didn't have a stdout");

    let stdout = tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut stdout = tokio::io::stdout();

        loop {
            if let Ok(bytes_read) = child_stdout.read(&mut buffer[..]).await {
                if bytes_read == 0 {
                    break;
                }
                if show_output {
                    if let Err(_) = stdout.write_all(&buffer[0..bytes_read]).await {
                        break;
                    }
                }
            } else {
                break;
            }
        }
    });

    let mut child_stderr = child.stderr.take().expect("Child didn't have a stderr");

    let stderr = tokio::spawn(async move {
        let mut buffer = [0; 1024];
        let mut stderr = tokio::io::stderr();
        loop {
            if let Ok(bytes_read) = child_stderr.read(&mut buffer[..]).await {
                if bytes_read == 0 {
                    break;
                }
                if show_output {
                    if let Err(_) = stderr.write_all(&buffer[0..bytes_read]).await {
                        break;
                    }
                }
            } else {
                break;
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
        exit_code: result.code().unwrap_or_else(|| -1),
        errors_corrected: 0,
    })
}
