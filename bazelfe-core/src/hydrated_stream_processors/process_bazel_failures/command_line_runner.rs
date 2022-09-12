use async_trait::async_trait;
use std::ffi::OsString;
use tokio::process::Command;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ExecutionResult {
    pub exit_success: bool,
    pub stdout: String,
    pub stderr: String,
}
#[async_trait]
pub trait CommandLineRunner: Clone + Send + Sync + std::fmt::Debug + 'static {
    async fn execute_command_line<S: Into<String> + Clone + Send>(
        &self,
        command_line: S,
    ) -> ExecutionResult;
}

#[derive(Clone, Debug)]
pub struct CommandLineRunnerImpl();

impl CommandLineRunnerImpl {
    fn decode_str(data: &Vec<u8>) -> String {
        if !data.is_empty() {
            std::str::from_utf8(data)
                .unwrap_or("Unable to decode content")
                .to_string()
        } else {
            String::from("")
        }
    }

    async fn execute_command_line<S: Into<String> + Clone + Send>(
        &self,
        command_line: S,
    ) -> ExecutionResult {
        let command_line = match shellwords::split(&command_line.into()) {
            Ok(command_line) => command_line,
            Err(err) => {
                return ExecutionResult {
                    exit_success: false,
                    stdout: String::default(),
                    stderr: err.to_string(),
                }
            }
        };

        let command_line: Vec<OsString> = { command_line.into_iter().map(|e| e.into()).collect() };

        if command_line.is_empty() {
            return ExecutionResult {
                exit_success: false,
                stdout: String::from(""),
                stderr: String::from("No command line supplied"),
            };
        };
        let mut cmd = Command::new(&command_line[0]);

        match cmd.args(&command_line[1..]).output().await {
            Ok(command_line_run_result) => {
                let exit_code = command_line_run_result.status.code().unwrap_or(-1);
                ExecutionResult {
                    stdout: CommandLineRunnerImpl::decode_str(&command_line_run_result.stdout),
                    stderr: CommandLineRunnerImpl::decode_str(&command_line_run_result.stderr),
                    exit_success: exit_code == 0,
                }
            }
            Err(err) => ExecutionResult {
                exit_success: false,
                stdout: String::default(),
                stderr: err.to_string(),
            },
        }
    }
}

#[async_trait]
impl CommandLineRunner for CommandLineRunnerImpl {
    async fn execute_command_line<S: Into<String> + Clone + Send>(
        &self,
        command_line: S,
    ) -> ExecutionResult {
        self.execute_command_line(command_line).await
    }
}

#[cfg(test)]
pub(crate) mod test_tools {
    use std::{collections::HashSet, sync::Arc};

    use tokio::sync::Mutex;

    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum ActionLogEntry {
        ExecuteCommandLine { command_line: String },
    }
    #[derive(Clone, Debug)]
    pub struct FakeCommandLineRunner {
        action_log: Arc<Mutex<Vec<ActionLogEntry>>>,
        command_line_fail_contains: HashSet<String>,
    }
    impl Default for FakeCommandLineRunner {
        fn default() -> Self {
            FakeCommandLineRunner::new(HashSet::new())
        }
    }
    impl FakeCommandLineRunner {
        pub fn new(command_line_fail_contains: HashSet<String>) -> Self {
            Self {
                action_log: Arc::new(Mutex::new(Vec::default())),
                command_line_fail_contains,
            }
        }
        pub async fn to_vec(&self) -> Vec<ActionLogEntry> {
            let locked = self.action_log.lock().await;
            (*locked).clone()
        }
    }

    #[async_trait::async_trait]
    impl CommandLineRunner for FakeCommandLineRunner {
        async fn execute_command_line<S: Into<String> + Clone + Send>(
            &self,
            command_line: S,
        ) -> ExecutionResult {
            let mut run_success = true;
            let command_line = command_line.into();
            for c in self.command_line_fail_contains.iter() {
                if command_line.contains(c) {
                    run_success = false;
                }
            }
            let mut lock = self.action_log.lock().await;
            lock.push(ActionLogEntry::ExecuteCommandLine { command_line });

            ExecutionResult {
                exit_success: run_success,
                stdout: String::default(),
                stderr: String::default(),
            }
        }
    }
}
