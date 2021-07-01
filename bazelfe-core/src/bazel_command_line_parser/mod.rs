mod options;
use std::{iter::Peekable, path::PathBuf};

pub use options::BuiltInAction;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BazelOption {
    BooleanOption(String, bool),
    OptionWithArg(String, String),
}

impl BazelOption {
    pub fn name(&self) -> &String {
        match self {
            BazelOption::BooleanOption(nme, _) => nme,
            BazelOption::OptionWithArg(nme, _) => nme,
        }
    }
    pub fn to_arg(&self) -> Vec<String> {
        match self {
            BazelOption::BooleanOption(nme, arg) => {
                if *arg {
                    vec![format!("--{}", nme)]
                } else {
                    vec![format!("--no{}", nme)]
                }
            }
            BazelOption::OptionWithArg(nme, arg) => {
                vec![format!("--{}", nme), arg.to_string()]
            }
        }
    }

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomAction {
    AutoTest,
}
impl CustomAction {
    pub fn action_for_options(&self) -> BuiltInAction {
        match self {
            CustomAction::AutoTest => BuiltInAction::Test,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    BuiltIn(BuiltInAction),
    Custom(CustomAction),
}

impl Action {
    pub fn action_for_options(&self) -> BuiltInAction {
        match self {
            Action::BuiltIn(b) => b.clone(),
            Action::Custom(c) => c.action_for_options(),
        }
    }
}

use std::str::FromStr;
impl FromStr for Action {
    type Err = ();

    fn from_str(input: &str) -> Result<Action, Self::Err> {
        if let Some(builtin) = input.parse::<BuiltInAction>().ok() {
            return Ok(Action::BuiltIn(builtin));
        }

        match input {
            "autotest" => Ok(Action::Custom(CustomAction::AutoTest)),
            _ => Err(()),
        }
    }
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommandLineParsingError {
    #[error("Command line invalid, path to bazel executable missing. ")]
    MissingBazelPath,

    #[error("Unknown argument {0}")]
    UnknownArgument(String),

    #[error("Missing the arg to option {0} when parsing command line")]
    MissingArgToOption(String),
}
#[derive(Error, Debug)]
pub enum ArgNormalizationError {
    #[error("Internal args cannot be passed to bazel sanely so are forbidden from this api, command line arg had: {0:?}")]
    HasInternalArg(CustomAction),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParsedCommandLine {
    pub bazel_binary: PathBuf,
    pub startup_options: Vec<BazelOption>,
    pub action: Option<Action>,
    pub action_options: Vec<BazelOption>,
    pub remaining_args: Vec<String>,
}
impl ParsedCommandLine {
    pub fn all_args_normalized(&self) -> Result<Vec<String>, ArgNormalizationError> {
        let mut result = Vec::default();

        result.extend(self.startup_options.iter().map(|e| e.to_arg()).flatten());

        if let Some(action) = &self.action {
            match action {
                Action::BuiltIn(b) => {
                    result.push(b.to_string());

                    result.extend(self.action_options.iter().map(|e| e.to_arg()).flatten());

                    if !self.remaining_args.is_empty() {
                        result.push(String::from("--"));
                        result.extend(self.remaining_args.iter().cloned());
                    }
                }
                Action::Custom(c) => {
                    return Err(ArgNormalizationError::HasInternalArg(c.clone()));
                }
            }
        } else {
            result.extend(self.remaining_args.iter().cloned());
        }
        Ok(result)
    }

    pub fn add_action_option_if_unset(&mut self, option: BazelOption) -> bool {
        if let Some(_) = self
            .action_options
            .iter()
            .find(|e| e.name() == option.name())
        {
            false
        } else {
            self.action_options.push(option);
            true
        }
    }

    pub fn is_action_option_set(&self, opt: &str) -> bool {
        self.action_options
            .iter()
            .find(|e| e.name() == opt)
            .is_some()
    }

    pub fn set_action(&mut self, action: Option<Action>) -> Option<Action> {
        let prev = self.action.take();
        if action.is_none() {
            self.action_options.clear();
        }
        self.action = action;

        prev
    }
}

fn extract_set_of_flags<'a, I: Iterator<Item = &'a String>>(
    iter: &mut Peekable<I>,
    flags: &Vec<BazelOption>,
) -> Result<Vec<BazelOption>, CommandLineParsingError> {
    let mut result: Vec<BazelOption> = Vec::default();
    'outer_loop: loop {
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
                        continue 'outer_loop;
                    }
                }

                for e in flags.iter() {
                    match e {
                        BazelOption::BooleanOption(nme, _) => {
                            if nme.as_str() == trimmed {
                                result.push(BazelOption::BooleanOption(nme.to_string(), true));
                                iter.next();
                                continue 'outer_loop;
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
                                    continue 'outer_loop;
                                } else {
                                    iter.next();
                                    if let Some(p) = iter.peek() {
                                        result.push(BazelOption::option_with_arg(
                                            nme.to_string(),
                                            p.to_string(),
                                        ));
                                        iter.next();
                                        continue 'outer_loop;
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

                // We found no matching option
                return Err(CommandLineParsingError::UnknownArgument(nxt.to_string()));
            } else {
                break 'outer_loop;
            }
        } else {
            break 'outer_loop;
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

    let startup_options = extract_set_of_flags(&mut command_line_iter, &options::STARTUP_OPTIONS)?;

    let action: Option<Action> = command_line_iter.peek().and_then(|cmd| cmd.parse().ok());

    if let Some(action) = action.as_ref() {
        command_line_iter.next();
        let options: Vec<BazelOption> = options::ACTION_TO_OPTIONS
            .get(&action.action_for_options())
            .expect("Should be impossible not to find options")
            .iter()
            .map(|&o| options::ALL_ACTION_OPTIONS[o].clone())
            .collect();
        let mut action_options = Vec::default();
        let mut action_args = Vec::default();
        'outer: loop {
            'inner: while let Some(&opt) = command_line_iter.peek() {
                if opt == "--" {
                    command_line_iter.next();
                    break 'outer;
                }
                if opt.starts_with("--") {
                    break 'inner;
                }
                action_args.push(opt.clone());
                command_line_iter.next();
            }
            let cur_options = extract_set_of_flags(&mut command_line_iter, &options)?;

            if cur_options.is_empty() {
                break 'outer;
            } else {
                action_options.extend(cur_options.into_iter());
            }
        }
        action_args.extend(command_line_iter.cloned());
        Ok(ParsedCommandLine {
            bazel_binary: bazel_path,
            startup_options: startup_options,
            action: Some(action.clone()),
            action_options: action_options,
            remaining_args: action_args,
        })
    } else {
        Ok(ParsedCommandLine {
            bazel_binary: bazel_path,
            startup_options: startup_options,
            action: None,
            action_options: Vec::default(),
            remaining_args: command_line_iter.cloned().collect(),
        })
    }
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

    #[tokio::test]
    async fn test_more_args() {
        let passthrough_command_line = vec![
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "--output_base=/tmp/foo build".to_string(),
            "test".to_string(),
            "--foo".to_string(),
            "bar".to_string(),
        ];

        let mut iter = passthrough_command_line.iter().peekable();
        let result = extract_set_of_flags(&mut iter, &options::STARTUP_OPTIONS)
            .expect("Should be able to parse the cmd line");

        let expected: Vec<BazelOption> = vec![
            BazelOption::OptionWithArg(String::from("host_jvm_args"), String::from("foobarbaz")),
            BazelOption::OptionWithArg(String::from("output_base"), String::from("/tmp/foo build")),
        ];

        assert_eq!(result, expected);

        let remaining: Vec<String> = iter.cloned().collect();
        let remaining_expected: Vec<String> =
            vec!["test".to_string(), "--foo".to_string(), "bar".to_string()];

        assert_eq!(remaining, remaining_expected);
    }

    #[tokio::test]
    async fn parse_bazel_command_line_1() {
        let passthrough_command_line = vec![
            "bazel".to_string(),
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "--output_base=/tmp/foo build".to_string(),
            "test".to_string(),
            "--keep_going".to_string(),
            "bar".to_string(),
        ];

        let result = parse_bazel_command_line(&passthrough_command_line)
            .expect("Should be able to parse the cmd line");

        let expected_startup_options: Vec<BazelOption> = vec![
            BazelOption::OptionWithArg(String::from("host_jvm_args"), String::from("foobarbaz")),
            BazelOption::OptionWithArg(String::from("output_base"), String::from("/tmp/foo build")),
        ];

        assert_eq!(result.startup_options, expected_startup_options);

        assert_eq!(result.action, Some(Action::BuiltIn(BuiltInAction::Test)));

        let expected_action_args: Vec<BazelOption> =
            vec![BazelOption::BooleanOption(String::from("keep_going"), true)];

        assert_eq!(result.action_options, expected_action_args);

        let remaining_expected: Vec<String> = vec!["bar".to_string()];

        assert_eq!(result.remaining_args, remaining_expected);

        let expected_args: Vec<String> = vec![
            "--host_jvm_args".to_string(),
            "foobarbaz".to_string(),
            "--output_base".to_string(),
            "/tmp/foo build".to_string(),
            "test".to_string(),
            "--keep_going".to_string(),
            "--".to_string(),
            "bar".to_string(),
        ];
        assert_eq!(
            result.all_args_normalized().expect("Can reproduce args"),
            expected_args
        );
    }

    #[tokio::test]
    async fn parse_bazel_command_line_2() {
        let passthrough_command_line =
            vec!["bazel".to_string(), "help".to_string(), "test".to_string()];

        let result = parse_bazel_command_line(&passthrough_command_line)
            .expect("Should be able to parse the cmd line");

        let expected_startup_options: Vec<BazelOption> = vec![];

        assert_eq!(result.startup_options, expected_startup_options);

        assert_eq!(result.action, Some(Action::BuiltIn(BuiltInAction::Help)));

        let expected_action_options: Vec<BazelOption> = vec![];
        assert_eq!(result.action_options, expected_action_options);

        let remaining_expected: Vec<String> = vec!["test".to_string()];

        assert_eq!(result.remaining_args, remaining_expected);

        let expected_args: Vec<String> =
            vec!["help".to_string(), "--".to_string(), "test".to_string()];
        assert_eq!(
            result.all_args_normalized().expect("Can reproduce args"),
            expected_args
        );
    }

    #[tokio::test]
    async fn parse_bazel_command_line_3() {
        let passthrough_command_line = vec![
            "bazel".to_string(),
            "build".to_string(),
            "--".to_string(),
            "foo/...".to_string(),
            "-foo/contrib/...".to_string(),
        ];

        let result = parse_bazel_command_line(&passthrough_command_line)
            .expect("Should be able to parse the cmd line");

        let expected_startup_options: Vec<BazelOption> = vec![];

        assert_eq!(result.startup_options, expected_startup_options);

        assert_eq!(result.action, Some(Action::BuiltIn(BuiltInAction::Build)));

        let expected_action_options: Vec<BazelOption> = vec![];

        assert_eq!(result.action_options, expected_action_options);

        let remaining_expected: Vec<String> =
            vec!["foo/...".to_string(), "-foo/contrib/...".to_string()];

        assert_eq!(result.remaining_args, remaining_expected);

        let expected_args: Vec<String> = vec![
            "build".to_string(),
            "--".to_string(),
            "foo/...".to_string(),
            "-foo/contrib/...".to_string(),
        ];
        assert_eq!(
            result.all_args_normalized().expect("Can reproduce args"),
            expected_args
        );
    }

    #[tokio::test]
    async fn parse_bazel_command_line_4() {
        let passthrough_command_line = vec![
            "bazel".to_string(),
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "--output_base=/tmp/foo build".to_string(),
            "test".to_string(),
            "bar".to_string(),
            "--keep_going".to_string(),
        ];

        let result = parse_bazel_command_line(&passthrough_command_line)
            .expect("Should be able to parse the cmd line");

        let expected_startup_options: Vec<BazelOption> = vec![
            BazelOption::OptionWithArg(String::from("host_jvm_args"), String::from("foobarbaz")),
            BazelOption::OptionWithArg(String::from("output_base"), String::from("/tmp/foo build")),
        ];

        assert_eq!(result.startup_options, expected_startup_options);

        assert_eq!(result.action, Some(Action::BuiltIn(BuiltInAction::Test)));

        let expected_action_options: Vec<BazelOption> =
            vec![BazelOption::BooleanOption(String::from("keep_going"), true)];

        assert_eq!(result.action_options, expected_action_options);

        let remaining_expected: Vec<String> = vec!["bar".to_string()];

        assert_eq!(result.remaining_args, remaining_expected);

        let expected_args: Vec<String> = vec![
            "--host_jvm_args".to_string(),
            "foobarbaz".to_string(),
            "--output_base".to_string(),
            "/tmp/foo build".to_string(),
            "test".to_string(),
            "--keep_going".to_string(),
            "--".to_string(),
            "bar".to_string(),
        ];
        assert_eq!(
            result.all_args_normalized().expect("Can reproduce args"),
            expected_args
        );
    }

    #[tokio::test]
    async fn unknown_arg_for_startup() {
        let passthrough_command_line = vec![
            "bazel".to_string(),
            "--unknown_arg=foo".to_string(),
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "--output_base=/tmp/foo build".to_string(),
            "test".to_string(),
            "bar".to_string(),
            "--keep_going".to_string(),
        ];

        match parse_bazel_command_line(&passthrough_command_line) {
            Ok(_) => panic!(
                "Should have failed to parse bad command line with an extra unknown startup option"
            ),
            Err(e) => match e {
                CommandLineParsingError::MissingBazelPath => {
                    panic!("Unexpected error, MissingBazelPath, not what we were testing");
                }
                CommandLineParsingError::UnknownArgument(o) => {
                    assert_eq!(o, "--unknown_arg=foo".to_string())
                }
                CommandLineParsingError::MissingArgToOption(_) => {
                    panic!("Unexpected error, MissingArgToOption, not what we were testing");
                }
            },
        }
    }

    #[tokio::test]
    async fn unknown_arg_for_action() {
        let passthrough_command_line = vec![
            "bazel".to_string(),
            "--host_jvm_args=\"foobarbaz\"".to_string(),
            "--output_base=/tmp/foo build".to_string(),
            "test".to_string(),
            "bar".to_string(),
            "--unknown_arg=foo".to_string(),
            "--keep_going".to_string(),
        ];

        match parse_bazel_command_line(&passthrough_command_line) {
            Ok(_) => panic!(
                "Should have failed to parse bad command line with an extra unknown startup option"
            ),
            Err(e) => match e {
                CommandLineParsingError::MissingBazelPath => {
                    panic!("Unexpected error, MissingBazelPath, not what we were testing");
                }
                CommandLineParsingError::UnknownArgument(o) => {
                    assert_eq!(o, "--unknown_arg=foo".to_string())
                }
                CommandLineParsingError::MissingArgToOption(_) => {
                    panic!("Unexpected error, MissingArgToOption, not what we were testing");
                }
            },
        }
    }
}
