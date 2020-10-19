use async_trait::async_trait;
use std::path::PathBuf;
use tokio::process::Command;

#[derive(Clone, PartialEq, Debug)]
pub struct ExecuteResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl std::convert::From<std::io::Error> for ExecuteResult {
    fn from(io_e: std::io::Error) -> Self {
        Self {
            exit_code: -120,
            stderr: format!("{:?}", io_e).to_string(),
            stdout: String::from(""),
        }
    }
}
pub type Result<T> = std::result::Result<T, ExecuteResult>;

#[async_trait]
pub trait BazelQuery {
    async fn execute(&self, args: &Vec<String>) -> ExecuteResult;
}

#[derive(Clone, Debug)]
pub struct BazelQueryBinaryImpl {
    bazel_executable_path: PathBuf,
}

pub fn from_binary_path(pb: PathBuf) -> BazelQueryBinaryImpl {
    BazelQueryBinaryImpl {
        bazel_executable_path: pb,
    }
}

impl BazelQueryBinaryImpl {
    fn decode_str(data: &Vec<u8>) -> String {
        if data.len() > 0 {
            std::str::from_utf8(&data)
                .unwrap_or("Unable to decode content")
                .to_string()
        } else {
            String::from("")
        }
    }
    async fn execute_command(&self, command: &Vec<String>) -> ExecuteResult {
        let mut cmd = Command::new(&self.bazel_executable_path);
        let command_result = match cmd.args(command).output().await {
            Err(e) => return e.into(),
            Ok(o) => o,
        };
        let exit_code = command_result.status.code().unwrap_or(-1);

        ExecuteResult {
            exit_code: exit_code,
            stdout: BazelQueryBinaryImpl::decode_str(&command_result.stdout),
            stderr: BazelQueryBinaryImpl::decode_str(&command_result.stderr),
        }
    }
}

#[async_trait]
impl BazelQuery for BazelQueryBinaryImpl {
    async fn execute(&self, args: &Vec<String>) -> ExecuteResult {
        self.execute_command(args).await
    }
}
