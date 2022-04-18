mod error_processor;
use std::path::PathBuf;

pub use error_processor::ErrorProcessor;
mod base_config;
pub use base_config::Config;

pub mod command_line_rewriter;
pub use command_line_rewriter::CommandLineRewriter;

pub mod daemon_config;
pub use daemon_config::DaemonConfig;

mod indexer_config;
pub use indexer_config::IndexerConfig;

pub fn parse_config(input: &str) -> Result<Config, toml::de::Error> {
    toml::from_str(input)
}

pub async fn load_config_file(
    command_line_path: &Option<&String>,
) -> Result<Config, Box<dyn std::error::Error>> {
    use std::str::FromStr;
    let mut path: Option<String> = None;
    if let Some(p) = command_line_path {
        let pbuf = PathBuf::from_str(p)?;
        if !pbuf.exists() {
            panic!("Expected to find config at path {}, but it didn't exist", p);
        }
        path = Some((*p).clone())
    };

    if path == None {
        if let Ok(home_dir) = std::env::var("HOME") {
            let cur_p = PathBuf::from(format!("{}/.bazelfe_config", home_dir));
            if cur_p.exists() {
                path = Some(cur_p.to_str().unwrap().to_string());
            }
        }
    }

    if let Some(path) = path {
        Ok(parse_config(&std::fs::read_to_string(path)?)?)
    } else {
        Ok(Config::default())
    }
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
}
