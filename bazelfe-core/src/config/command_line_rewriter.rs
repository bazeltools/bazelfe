use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct EmptyTestToLocalRepoCfg {
    #[serde(default = "default_command_to_use")]
    pub command_to_use: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct SuggestTestTargetConfig {
    #[serde(default = "default_distance_to_use")]
    pub distance_to_expand: u32,
}

impl Default for EmptyTestToLocalRepoCfg {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_command_to_use() -> String {
    String::from("//...")
}

fn default_distance_to_use() -> u32 {
    2
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum TestActionMode {
    EmptyTestToLocalRepo(EmptyTestToLocalRepoCfg),
    EmptyTestToFail,
    SuggestTestTarget(SuggestTestTargetConfig),
    Passthrough,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct CommandLineRewriter {
    #[serde(default = "default_test_rewrite_mode")]
    pub test: TestActionMode,
}

impl Default for CommandLineRewriter {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn default_test_rewrite_mode() -> TestActionMode {
    TestActionMode::Passthrough
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn with_command_specified() {
        let command_line_rewriter: CommandLineRewriter = toml::from_str(
            r#"
        [test]
            type = 'EmptyTestToLocalRepo'
            command_to_use = "foo"
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            CommandLineRewriter {
                test: TestActionMode::EmptyTestToLocalRepo(EmptyTestToLocalRepoCfg {
                    command_to_use: String::from("foo")
                })
            }
        );
    }

    #[test]
    fn without_command_specified() {
        let command_line_rewriter: CommandLineRewriter = toml::from_str(
            r#"
        [test]
            type = 'EmptyTestToLocalRepo'
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            CommandLineRewriter {
                test: TestActionMode::EmptyTestToLocalRepo(EmptyTestToLocalRepoCfg {
                    command_to_use: String::from("//...")
                })
            }
        );
    }

    #[test]
    fn type_with_no_config_options() {
        let command_line_rewriter: CommandLineRewriter = toml::from_str(
            r#"
        [test]
            type = 'EmptyTestToFail'
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            CommandLineRewriter {
                test: TestActionMode::EmptyTestToFail
            }
        );
    }

    #[test]
    fn empty_config() {
        let command_line_rewriter: CommandLineRewriter = toml::from_str(
            r#"
        "#,
        )
        .unwrap();

        assert_eq!(
            command_line_rewriter,
            CommandLineRewriter {
                test: TestActionMode::Passthrough
            }
        );
    }
}
