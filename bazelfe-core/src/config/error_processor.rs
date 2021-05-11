use serde::{Deserialize, Deserializer};

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

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_simple_parse() {
        let error_processor: ErrorProcessor = toml::from_str(
            r#"
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
            error_processor,
            ErrorProcessor {
                name: String::from("Identifying unused proto imports"),
                active_action_type: String::from("proto_library"),
                run_on_success: false,
                regex_match: String::from(r#"^(.*):(\d+):(\d+): warning: Import (.*) is unused.$"#),
                target_command_line: String::from(r#""/bin/foo" '$1' "$2" "$3""#)
            }
        );
    }
}
