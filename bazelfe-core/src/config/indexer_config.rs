use serde::{ser::SerializeSeq, Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct IndexerConfig {
    #[serde(default = "default_blacklist_rule_kind")]
    pub blacklist_rule_kind: Vec<String>,
}

fn default_blacklist_rule_kind() -> Vec<String> {
    Vec::default()
}

impl Default for IndexerConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn with_command_specified() {
        let command_line_rewriter: IndexerConfig = toml::from_str(
            r#"
            blacklist_rule_kind = ["foo"]
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            IndexerConfig {
                blacklist_rule_kind: vec![String::from("foo")],
            }
        );
    }
}
