mod options;
use std::{iter::Peekable, path::PathBuf};

pub use options::BuiltInAction;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BazelOption {
    BooleanOption(String, bool),
    OptionWithArg(String, String),
}
impl BazelOption {
    pub fn option_with_arg(name: String, arg: String) -> BazelOption {
        let mut min_pos = i32::MAX;
        let mut max_pos = -1;
        for (idx, chr) in arg.chars().enumerate() {
            let idx = idx as i32;
            if chr == '"' {
                if idx < min_pos {
                    min_pos = idx;
                }

                if idx > max_pos {
                    max_pos = idx;
                }
            }
        }
        if min_pos == 0 && max_pos == (arg.len() - 1) as i32 {
            let arg = &arg[1..(arg.len() - 1)];
            BazelOption::OptionWithArg(name, arg.to_string())
        } else {
            BazelOption::OptionWithArg(name, arg)
        }
    }
}

#[derive(Debug, Clone)]
pub enum CustomAction {
    AutoTest,
}

#[derive(Debug, Clone)]
pub enum Action {
    BuiltIn(BuiltInAction),
    Custom(CustomAction),
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandLineParsingError {
    // #[error("Reporting user error: `{0}`")]
    // UserErrorReport(super::UserReportError),
    // #[error(transparent)]
    // CommandLineRewriterActionError(command_line_rewriter_action::RewriteCommandLineError),
    #[error("Command line invalid, path to bazel executable missing. ")]
    MissingBazelPath,

    #[error("Missing the arg to option {0} when parsing command line")]
    MissingArgToOption(String),
    // #[error("Unclassified or otherwise unknown error occured: `{0}`")]
    // Unknown(Box<dyn std::error::Error>),
}

pub struct ParsedCommandLine {
    pub bazel_binary: PathBuf,
    pub startup_options: Vec<BazelOption>,
    pub action: Option<String>,
    pub remaining_args: Vec<String>,
}

fn extract_set_of_flags<'a, I: Iterator<Item = &'a String>>(
    iter: &mut Peekable<I>,
    flags: &Vec<BazelOption>,
) -> Result<Vec<BazelOption>, CommandLineParsingError> {
    let mut result: Vec<BazelOption> = Vec::default();
    loop {
        let peek_str = iter.peek().cloned();
        if let Some(nxt) = peek_str {
            let mut trimmed = nxt.trim();

            if trimmed.starts_with("--") {
                let mut value: Option<String> = None;
                trimmed = &trimmed[2..];
                if let Some(loc_eq) = trimmed.find("=") {
                    let (prev, mut post) = trimmed.split_at(loc_eq);
                    post = &post[1..];
                    value = Some(post.to_string());
                    trimmed = prev;
                } else if trimmed.starts_with("no") {
                    let t = &trimmed[2..];
                    let boolean_option_found = flags.iter().find_map(|e| match e {
                        BazelOption::BooleanOption(nme, _) => {
                            if nme.as_str() == t {
                                Some(BazelOption::BooleanOption(nme.to_string(), false))
                            } else {
                                None
                            }
                        }
                        BazelOption::OptionWithArg(_, _) => None,
                    });
                    if let Some(boolean) = boolean_option_found {
                        result.push(boolean);
                        iter.next();
                        continue;
                    }
                }

                for e in flags.iter() {
                    match e {
                        BazelOption::BooleanOption(nme, _) => {
                            if nme.as_str() == trimmed {
                                result.push(BazelOption::BooleanOption(nme.to_string(), true));
                                iter.next();
                                continue;
                            }
                        }
                        BazelOption::OptionWithArg(nme, _) => {
                            if nme.as_str() == trimmed {
                                if let Some(v) = value.as_ref() {
                                    result.push(BazelOption::option_with_arg(
                                        nme.to_string(),
                                        v.to_string(),
                                    ));
                                    iter.next();
                                    continue;
                                } else {
                                    iter.next();
                                    if let Some(p) = iter.peek() {
                                        result.push(BazelOption::option_with_arg(
                                            nme.to_string(),
                                            p.to_string(),
                                        ));
                                        iter.next();
                                        continue;
                                    } else {
                                        return Err(CommandLineParsingError::MissingArgToOption(
                                            nme.clone(),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                break;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    Ok(result)
}

pub fn parse_bazel_command_line(
    command_line: &Vec<String>,
) -> Result<ParsedCommandLine, CommandLineParsingError> {
    let mut command_line_iter = command_line.iter().peekable();
    let bazel_path = if let Some(p) = command_line_iter.next() {
        PathBuf::from(p)
    } else {
        return Err(CommandLineParsingError::MissingBazelPath);
    };

    extract_set_of_flags(&mut command_line_iter, &options::STARTUP_OPTIONS)?;

    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_no_args() {
        let passthrough_command_line =
            vec!["test".to_string(), "--foo".to_string(), "bar".to_string()];

        let mut iter = passthrough_command_line.iter().peekable();
        let result = extract_set_of_flags(&mut iter, &options::STARTUP_OPTIONS)
            .expect("Should be able to parse the cmd line");
        let expected: Vec<BazelOption> = Vec::default();
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_some_args() {
        let passthrough_command_line = vec![
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "test".to_string(),
            "--foo".to_string(),
            "bar".to_string(),
        ];

        let mut iter = passthrough_command_line.iter().peekable();
        let result = extract_set_of_flags(&mut iter, &options::STARTUP_OPTIONS)
            .expect("Should be able to parse the cmd line");

        let expected: Vec<BazelOption> = vec![BazelOption::OptionWithArg(
            String::from("host_jvm_args"),
            String::from("foobarbaz"),
        )];

        assert_eq!(result, expected);

        let remaining: Vec<String> = iter.cloned().collect();
        let remaining_expected: Vec<String> =
            vec!["test".to_string(), "--foo".to_string(), "bar".to_string()];

        assert_eq!(remaining, remaining_expected);
    }
}
