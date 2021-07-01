use super::error_processor::ErrorProcessor;
use super::{command_line_rewriter::CommandLineRewriter, DaemonConfig};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Config {
    /// List of custom user processors to run over the stdout/stderr streams
    #[serde(alias = "ErrorProcessors")]
    pub error_processors: Option<Vec<ErrorProcessor>>,
    /// Where to load/store the index on disk
    /// In several use cases this might be dynamically fetched/generated, this can be overridden on the command line.
    pub index_input_location: Option<std::path::PathBuf>,

    /// Where to find buildozer on disk
    pub buildozer_path: Option<std::path::PathBuf>,

    /// where to bind the local port to listen to BES from bazel.
    /// If left empty this will default to a random port on localhost.
    #[serde(default, deserialize_with = "parse_bes_bind_address")]
    pub bes_server_bind_address: Option<std::net::SocketAddr>,

    /// This controls if we should print out what actions were taken by bazelfe in the event the final build could be repaired.
    /// If we took actions and the build still failed we report the actions always as to better inform the user of the state of the tree.
    #[serde(default)]
    pub disable_action_stories_on_success: bool,

    #[serde(
        rename = "CommandLineRewriter",
        default = "CommandLineRewriter::default"
    )]
    pub command_line_rewriter: CommandLineRewriter,

    #[serde(rename = "DaemonConfig", default = "DaemonConfig::default")]
    pub daemon_config: DaemonConfig,
}

// We want to use the serde configured defaults for our default implemenation to not be
// building up two separate paths.
impl Default for Config {
    fn default() -> Self {
        super::parse_config("").unwrap()
    }
}

fn parse_bes_bind_address<'de, D>(deserializer: D) -> Result<Option<std::net::SocketAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<&str> = Deserialize::deserialize(deserializer)?;

    if let Some(s) = s {
        s.parse().map_err(serde::de::Error::custom).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_simple_parse() {
        let config: Config = toml::from_str(
            r#"
        [[error_processors]]
        name = "Identifying unused proto imports"
        active_action_type = "proto_library"
        regex_match =  '^(.*):(\d+):(\d+): warning: Import (.*) is unused.$'
        target_command_line = '''
            "/bin/foo" '$1' "$2" "$3"
        '''
        "#,
        )
        .unwrap();

        assert_eq!(
            config.error_processors,
            Some(vec![ErrorProcessor {
                name: String::from("Identifying unused proto imports"),
                active_action_type: String::from("proto_library"),
                run_on_success: false,
                regex_match: String::from(r#"^(.*):(\d+):(\d+): warning: Import (.*) is unused.$"#),
                target_command_line: String::from(r#""/bin/foo" '$1' "$2" "$3""#)
            }])
        );
    }

    #[test]
    fn test_empty_parse() {
        let config: Config = toml::from_str("").unwrap();

        assert_eq!(config.error_processors, None);
    }
}
