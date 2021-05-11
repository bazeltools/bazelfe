use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct DaemonConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_communication_folder")]
    pub daemon_communication_folder: PathBuf,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_enabled() -> bool {
    false
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
                daemon_communication_folder: default_communication_folder()
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
                enabled: true,
                daemon_communication_folder: PathBuf::from("/tmp/foo")
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
                enabled: true,
                daemon_communication_folder: default_communication_folder()
            }
        );
    }
}
