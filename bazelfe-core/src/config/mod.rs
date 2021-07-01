mod error_processor;
pub use error_processor::ErrorProcessor;
mod base_config;
pub use base_config::Config;

pub mod command_line_rewriter;
pub use command_line_rewriter::CommandLineRewriter;

pub mod daemon_config;
pub use daemon_config::DaemonConfig;

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
}
