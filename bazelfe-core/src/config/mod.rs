use serde::{Deserialize, Deserializer};
use serde_derive::Deserialize;

#[derive(Deserialize, Default, Debug, PartialEq, Eq)]
pub struct Config {
    pub error_processors: Option<Vec<ErrorProcessor>>,
    pub index_input_location: Option<std::path::PathBuf>,
    pub buildozer_path: Option<std::path::PathBuf>,
    #[serde(default, deserialize_with = "parse_bes_bind_address")]
    pub bes_server_bind_address: Option<std::net::SocketAddr>,
    #[serde(default)]
    pub disable_action_stories_on_success: bool,
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
fn clean_command_line<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(deserializer)?;
    Ok(s.lines().map(|ln| ln.trim_start()).collect::<String>())
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ErrorProcessor {
    pub name: String,
    pub active_action_type: String,
    #[serde(default)]
    pub run_on_success: bool,
    pub regex_match: String,
    #[serde(deserialize_with = "clean_command_line")]
    pub target_command_line: String,
}

pub fn parse_config(input: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(input)
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_simple_parse() {
        let config: Config = super::parse_config(
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
        let config: Config = super::parse_config("").unwrap();

        assert_eq!(config.error_processors, None);
    }
}
