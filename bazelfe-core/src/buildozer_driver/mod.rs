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
            stderr: format!("{:?}", io_e),
            stdout: String::from(""),
        }
    }
}
pub type Result<T> = std::result::Result<T, ExecuteResultError>;

#[derive(Debug, Clone, PartialOrd, PartialEq, Eq)]
pub enum BazelAttrTarget {
    Deps,
    RuntimeDeps,
    Other(String),
}

impl BazelAttrTarget {
    pub fn as_str<'a>(self: &'a Self) -> &'a str {
        match self {
            BazelAttrTarget::Deps => "deps",
            BazelAttrTarget::RuntimeDeps => "runtime_deps",
            BazelAttrTarget::Other(o) => o.as_str(),
        }
    }
}
#[async_trait]
pub trait Buildozer: Clone + Send + Sync + std::fmt::Debug + 'static {
    async fn print_attr(&self, attr: &BazelAttrTarget, label: &String) -> Result<Vec<String>>;
    async fn add_to(
        &self,
        to_what: &BazelAttrTarget,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()>;

    async fn remove_from(
        &self,
        from_what: &BazelAttrTarget,
        target_to_operate_on: &String,
        label_to_remove: &String,
    ) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct BuildozerBinaryImpl {
    buildozer_executable_path: PathBuf,
}

pub fn from_binary_path(pb: &PathBuf) -> BuildozerBinaryImpl {
    BuildozerBinaryImpl {
        buildozer_executable_path: pb.clone(),
    }
}

impl BuildozerBinaryImpl {
    fn decode_str(data: &Vec<u8>) -> String {
        if !data.is_empty() {
            std::str::from_utf8(data)
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
                exit_code,
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
    async fn print_attr(&self, attr: &BazelAttrTarget, label: &String) -> Result<Vec<String>> {
        let (_raw_args, cmd_result) = self
            .execute_command(vec![
                &format!("print {}", attr.as_str()),
                &crate::label_utils::sanitize_label(label.clone()),
            ])
            .await?;

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
            .map(crate::label_utils::sanitize_label)
            .collect())
    }

    async fn add_to(
        &self,
        to_what: &BazelAttrTarget,
        target_to_operate_on: &String,
        label_to_add: &String,
    ) -> Result<()> {
        let buildozer_cmd = format!("add {} {}", to_what.as_str(), label_to_add);
        // buildozer 'add deps //base' //pkg:rule //pkg:rule2
        let _ = self
            .execute_command(vec![
                &buildozer_cmd,
                &crate::label_utils::sanitize_label(target_to_operate_on.clone()),
            ])
            .await?;
        Ok(())
    }

    async fn remove_from(
        &self,
        from_what: &BazelAttrTarget,
        target_to_operate_on: &String,
        label_to_remove: &String,
    ) -> Result<()> {
        let buildozer_cmd = format!("remove {} {}", from_what.as_str(), label_to_remove);
        // buildozer 'add deps //base' //pkg:rule //pkg:rule2
        let _ = self
            .execute_command(vec![
                &buildozer_cmd,
                &crate::label_utils::sanitize_label(target_to_operate_on.clone()),
            ])
            .await?;
        Ok(())
    }
}
