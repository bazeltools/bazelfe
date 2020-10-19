use std::ffi::OsString;
use std::process::Stdio;
use std::sync::atomic::Ordering;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use tokio::process::Command;

static SUB_PROCESS_PID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

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

fn update_command<S: Into<String> + Clone>(
    command: &Vec<S>,
    srv_port: u16,
) -> Option<Vec<OsString>> {
    let lst_str: Vec<String> = command.iter().skip(1).map(|e| e.clone().into()).collect();

    let mut idx = 0;
    let mut do_continue = true;
    while idx < lst_str.len() && do_continue {
        if !lst_str[idx].starts_with("--") {
            do_continue = false
        } else {
            idx += 1
        }
    }

    if do_continue == true {
        return None;
    }

    let command_element: &str = &lst_str[idx].to_lowercase();

    match command_element {
        "build" => (),
        "test" => (),
        _ => return None,
    };

    let (pre_cmd, cmd_including_post) = lst_str.split_at(idx);
    let (cmd, post_command) = cmd_including_post.split_at(1);

    let bes_section = vec![
        cmd[0].clone(),
        String::from("--build_event_publish_all_actions"),
        String::from("--experimental_build_event_upload_strategy=local"),
        String::from("--build_event_text_file_path_conversion"),
        String::from("--color"),
        String::from("yes"),
        String::from("--bes_backend"),
        String::from(format!("grpc://127.0.0.1:{}", srv_port)),
    ];

    Some(
        vec![pre_cmd.iter(), bes_section.iter(), post_command.iter()]
            .into_iter()
            .flat_map(|e| e)
            .map(|e| e.into())
            .collect(),
    )
}

#[derive(Clone, PartialEq, Debug)]
pub struct ExecuteResult {
    pub exit_code: i32,
    pub errors_corrected: u32,
}
pub async fn execute_bazel<S: Into<String> + Clone>(
    command: Vec<S>,
    bes_port: u16,
) -> ExecuteResult {
    execute_bazel_output_control(command, bes_port, true).await
}

pub async fn execute_bazel_output_control<S: Into<String> + Clone>(
    command: Vec<S>,
    bes_port: u16,
    show_output: bool,
) -> ExecuteResult {
    let application: OsString = command
        .first()
        .map(|a| {
            let a: String = a.clone().into();
            a
        })
        .expect("Should have had at least one arg the bazel process itself.")
        .into();

    let updated_command = match update_command(&command, bes_port) {
        Some(e) => e,
        None => command
            .iter()
            .skip(1)
            .map(|str_ref| {
                let a: String = str_ref.clone().into();
                let a: OsString = a.into();
                a
            })
            .collect(),
    };

    debug!("{:?} {:?}", application, updated_command);
    let mut cmd = Command::new(application);

    cmd.args(&updated_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to start bazel process");
    SUB_PROCESS_PID.store(child.id(), Ordering::SeqCst);

    let mut child_stdout = child.stdout.take().expect("Child didn't have a stdout");

    tokio::spawn(async move {
        let mut bytes_read = 1;
        let mut buffer = [0; 1024];
        let mut stdout = tokio::io::stdout();

        while bytes_read > 0 {
            bytes_read = child_stdout.read(&mut buffer[..]).await.unwrap();
            if show_output {
                stdout.write_all(&buffer[0..bytes_read]).await.unwrap();
            }
        }
    });

    let mut child_stderr = child.stderr.take().expect("Child didn't have a stderr");

    tokio::spawn(async move {
        let mut bytes_read = 1;
        let mut buffer = [0; 1024];
        let mut stderr = tokio::io::stderr();
        while bytes_read > 0 {
            bytes_read = child_stderr.read(&mut buffer[..]).await.unwrap();
            if show_output {
                stderr.write_all(&buffer[0..bytes_read]).await.unwrap();
            }
        }
    });
    let result = child.await.expect("The command wasn't running");

    SUB_PROCESS_PID.store(0, Ordering::SeqCst);

    ExecuteResult {
        exit_code: result.code().unwrap_or_else(|| -1),
        errors_corrected: 0,
    }
}
pub mod action_event_stream;
pub mod expand_target_to_guesses;
pub mod process_build_abort_errors;
pub mod process_missing_dependency_errors;
mod sanitization_tools;
