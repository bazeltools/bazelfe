use std::path::PathBuf;

use regex::Regex;
use serde::{ser::SerializeSeq, Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct NotifyRegexes(pub Vec<Regex>);
impl PartialEq for NotifyRegexes {
    fn eq(&self, other: &Self) -> bool {
        let mut a: Vec<String> = self.0.iter().map(|e| e.to_string()).collect();
        let mut b: Vec<String> = other.0.iter().map(|e| e.to_string()).collect();

        a.sort();
        b.sort();
        a == b
    }
}
impl Eq for NotifyRegexes {}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct DaemonConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_communication_folder")]
    pub daemon_communication_folder: PathBuf,

    #[serde(
        default = "default_inotify_ignore",
        deserialize_with = "parse_regex",
        serialize_with = "serialize_regex"
    )]
    pub inotify_ignore_regexes: NotifyRegexes,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_enabled() -> bool {
    false
}

fn serialize_regex<'de, S>(regexes: &NotifyRegexes, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let mut s = serializer.serialize_seq(Some(regexes.0.len()))?;
    for r in regexes.0.iter() {
        s.serialize_element(&r.to_string())?;
    }
    s.end()
}

fn parse_regex<'de, D>(deserializer: D) -> Result<NotifyRegexes, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Vec<String> = Deserialize::deserialize(deserializer)?;

    let mut res: Vec<Regex> = Vec::default();

    for e in s {
        let cur: Regex = Regex::new(&e).map_err(serde::de::Error::custom)?;
        res.push(cur);
    }
    Ok(NotifyRegexes(res))
}

fn default_inotify_ignore() -> NotifyRegexes {
    NotifyRegexes(vec![
        Regex::new("bazel-.*").expect("Constant known good regex")
    ])
}

/// Default communication is a folder under tmp thats namespaced based on the CWD hashed.
fn default_communication_folder() -> PathBuf {
    let current_path = std::env::current_dir().expect("Should be able to get current folder");
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(current_path.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    std::env::temp_dir()
        .join("bazelfe")
        .join("daemons")
        .join(format!("{:x}", result))
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn with_command_specified() {
        let command_line_rewriter: DaemonConfig = toml::from_str(
            r#"
        enabled = false
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            DaemonConfig {
                enabled: false,
                daemon_communication_folder: default_communication_folder(),
                inotify_ignore_regexes: default_inotify_ignore()
            }
        );
    }

    #[test]
    fn with_communication_path_specified() {
        let command_line_rewriter: DaemonConfig = toml::from_str(
            r#"
            daemon_communication_folder = "/tmp/foo"
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            DaemonConfig {
                enabled: false,
                daemon_communication_folder: PathBuf::from("/tmp/foo"),
                inotify_ignore_regexes: default_inotify_ignore()
            }
        );
    }

    #[test]
    fn empty_config() {
        let command_line_rewriter: DaemonConfig = toml::from_str(
            r#"
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            DaemonConfig {
                enabled: false,
                daemon_communication_folder: default_communication_folder(),
                inotify_ignore_regexes: default_inotify_ignore()
            }
        );
    }
}
