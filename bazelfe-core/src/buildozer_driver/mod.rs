use bazelfe_protos::*;

use ::prost::Message;
use async_trait::async_trait;
use std::{ffi::OsString, path::PathBuf};
use tokio::process::Command;

#[derive(Clone, PartialEq, Debug)]
pub struct ExecuteResultError {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl std::convert::From<std::io::Error> for ExecuteResultError {
    fn from(io_e: std::io::Error) -> Self {
        Self {
            exit_code: -120,
            stderr: format!("{:?}", io_e).to_string(),
            stdout: String::from(""),
        }
    }
}
pub type Result<T> = std::result::Result<T, ExecuteResultError>;

// #[derive(Clone, PartialEq, Debug)]
// pub enum ExecuteResult {
//     Error(ExecuteResultError),
//     Success(devtools::buildozer::Output),
// }

#[async_trait]
pub trait Buildozer {
    async fn print_deps(&self, label: &String) -> Result<Vec<String>>;
    async fn add_dependency(
        &self,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()>;

    async fn remove_dependency(
        &self,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct BuildozerBinaryImpl {
    buildozer_executable_path: PathBuf,
}

pub fn from_binary_path(pb: PathBuf) -> BuildozerBinaryImpl {
    BuildozerBinaryImpl {
        buildozer_executable_path: pb,
    }
}

impl BuildozerBinaryImpl {
    fn decode_str(data: &Vec<u8>) -> String {
        if data.len() > 0 {
            std::str::from_utf8(&data)
                .unwrap_or("Unable to decode content")
                .to_string()
        } else {
            String::from("")
        }
    }
    async fn execute_command<S: Into<String> + Clone>(
        &self,
        command: Vec<S>,
    ) -> Result<(Vec<OsString>, devtools::buildozer::Output)> {
        let command: Vec<OsString> = {
            let v = vec![String::from("--output_proto")];

            v.into_iter()
                .chain(command.into_iter().map(|e| e.into()))
                .map(|e| e.into())
                .collect()
        };

        let mut cmd = Command::new(&self.buildozer_executable_path);

        let command_result = cmd.args(&command).output().await?;

        let exit_code = command_result.status.code().unwrap_or(-1);
        if exit_code < 0 {
            return Err(ExecuteResultError {
                exit_code: exit_code,
                stdout: BuildozerBinaryImpl::decode_str(&command_result.stdout),
                stderr: BuildozerBinaryImpl::decode_str(&command_result.stderr),
            });
        }

        let out = devtools::buildozer::Output::decode(&*command_result.stdout).unwrap();
        Ok((command, out))
    }
}

#[async_trait]
impl Buildozer for BuildozerBinaryImpl {
    async fn print_deps(&self, label: &String) -> Result<Vec<String>> {
        let (_raw_args, cmd_result) = self.execute_command(vec!["print deps", &label]).await?;

        Ok(cmd_result
            .records
            .into_iter()
            .flat_map(|record| {
                record.fields.into_iter().flat_map(|f| match f.value {
                    None => vec![].into_iter(),
                    Some(v) => match v {
                        devtools::buildozer::output::record::field::Value::List(lst) => {
                            lst.strings.into_iter()
                        }
                        devtools::buildozer::output::record::field::Value::Text(e) => {
                            vec![e].into_iter()
                        }
                        devtools::buildozer::output::record::field::Value::Number(num) => {
                            panic!("Unexpected number entry: {:?}", num)
                        }
                        devtools::buildozer::output::record::field::Value::Error(_) => {
                            // This happens if the deps aren't present, its not meaningful :/
                            vec![].into_iter()
                        }
                    },
                })
            })
            .collect())
    }

    async fn add_dependency(
        &self,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()> {
        // buildozer 'add deps //base' //pkg:rule //pkg:rule2
        let _ = self
            .execute_command(vec![
                &format!("add deps {}", label_to_add),
                &target_to_operate_on,
            ])
            .await?;
        Ok(())
    }

    async fn remove_dependency(
        &self,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()> {
        // buildozer 'add deps //base' //pkg:rule //pkg:rule2
        let _ = self
            .execute_command(vec![
                &format!("remove deps {}", label_to_add),
                &target_to_operate_on,
            ])
            .await?;
        Ok(())
    }
}
