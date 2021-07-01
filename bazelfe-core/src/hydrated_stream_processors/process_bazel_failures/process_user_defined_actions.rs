use crate::config::{Config, ErrorProcessor};

use crate::build_events::hydrated_stream;
use regex::Regex;
use std::time::Instant;
use std::{collections::HashMap, sync::Arc};

use super::{
    command_line_runner::CommandLineRunner,
    shared_utils::{text_logs_from_failure, text_logs_from_success},
};

#[derive(Clone, Debug)]
pub struct UserDefinedActionsStateCache {
    run_always: HashMap<String, Vec<Arc<(Regex, ErrorProcessor)>>>,
    failure_only_action: HashMap<String, Vec<Arc<(Regex, ErrorProcessor)>>>,
}

impl UserDefinedActionsStateCache {
    pub fn from_config(
        config: &Config,
    ) -> Result<UserDefinedActionsStateCache, Box<dyn std::error::Error>> {
        let mut failure_only: HashMap<String, Vec<Arc<(Regex, ErrorProcessor)>>> =
            HashMap::default();
        let mut run_always: HashMap<String, Vec<Arc<(Regex, ErrorProcessor)>>> = HashMap::default();
        for ep in config
            .error_processors
            .as_ref()
            .unwrap_or(&Vec::default())
            .iter()
        {
            let ep = ep.clone();
            let regexp = Regex::new(&ep.regex_match)?;
            let r = if ep.run_on_success {
                &mut run_always
            } else {
                &mut failure_only
            };
            let entry = r.entry(ep.active_action_type.clone());
            let vec: &mut Vec<Arc<(Regex, ErrorProcessor)>> = entry.or_default();
            vec.push(Arc::new((regexp, ep)));
        }
        Ok(UserDefinedActionsStateCache {
            run_always: run_always,
            failure_only_action: failure_only,
        })
    }
}

#[derive(Clone, PartialEq, Debug)]
struct CommandLineAction {
    pub config_name: String,
    pub label: String,
    pub command_line: String,
    pub why: String,
}

fn extract_configured_regexes<'a, 'b, 'c>(
    target_label: &'a String,
    input_error_streams: &'a Vec<String>,
    command_stream: &'a mut Vec<CommandLineAction>,
    process_state: &Vec<Arc<(Regex, ErrorProcessor)>>,
) {
    for stream in input_error_streams {
        for ln in stream.lines() {
            for e in process_state {
                let (regex, ep) = e.as_ref();
                let captures = regex.captures(ln);
                match captures {
                    None => (),
                    Some(captures) => {
                        let fmt_map: Vec<String> = captures
                            .iter()
                            .filter_map(|cap| cap.map(|e| e.as_str().to_string()))
                            .collect();
                        if !fmt_map.is_empty() {
                            use dynfmt::{Format, SimpleCurlyFormat};

                            let formatted = SimpleCurlyFormat
                                .format(&ep.target_command_line, &fmt_map)
                                .unwrap()
                                .to_owned()
                                .to_string();

                            let correction = CommandLineAction {
                                label: target_label.clone(),
                                config_name: ep.name.clone(),
                                command_line: formatted,
                                why: String::from("Triggered from user config"),
                            };
                            command_stream.push(correction);
                        }
                    }
                }
            }
        }
    }
}

async fn apply_candidates<T: CommandLineRunner + Clone + Send + Sync + 'static>(
    candidate_correction_commands: Vec<CommandLineAction>,
    command_line_runner: T,
) -> super::Response {
    let mut target_stories = Vec::default();
    if candidate_correction_commands.len() == 0 {
        return super::Response::new(Vec::default());
    }
    for correction_command in candidate_correction_commands.into_iter() {
        debug!("Running user action:\n {:?}", correction_command);
        let execute_res = command_line_runner
            .execute_command_line(&correction_command.command_line)
            .await;

        target_stories.push(super::TargetStory {
            target: correction_command.label.clone(),
            action: super::TargetStoryAction::RanUserAction {
                user_action_name: correction_command.config_name.clone(),
                why: correction_command.why.clone(),
                command_line: correction_command.command_line.clone(),
                execution_result: execute_res,
            },
            when: Instant::now(),
        });
    }
    super::Response::new(target_stories)
}

pub async fn process_action_failed<T: CommandLineRunner + Clone + Send + Sync + 'static>(
    command_line_runner: T,
    action_failed_info: &hydrated_stream::ActionFailedErrorInfo,
    user_defined_action_state: &UserDefinedActionsStateCache,
) -> super::Response {
    if let Some(tpe) = &action_failed_info.target_kind {
        let failure_action_data = user_defined_action_state.failure_only_action.get(tpe);
        let remaining_action_data = user_defined_action_state.run_always.get(tpe);

        let action_data = failure_action_data
            .and_then(|e| remaining_action_data.map(|r| e.iter().chain(r.iter())));
        if let Some(action_data) = action_data {
            let action_data: Vec<Arc<(Regex, ErrorProcessor)>> = action_data.cloned().collect();
            if !action_data.is_empty() {
                let mut candidate_correction_commands: Vec<CommandLineAction> = vec![];
                let error_streams = text_logs_from_failure(action_failed_info).await;
                extract_configured_regexes(
                    &action_failed_info.label,
                    &error_streams,
                    &mut candidate_correction_commands,
                    &action_data,
                );
                return apply_candidates(candidate_correction_commands, command_line_runner).await;
            }
        }
    }
    super::Response::new(Vec::default())
}

pub async fn process_action_success<T: CommandLineRunner + Clone + Send + Sync + 'static>(
    command_line_runner: T,
    action_success_info: &hydrated_stream::ActionSuccessInfo,
    user_defined_action_state: &UserDefinedActionsStateCache,
) -> super::Response {
    if let Some(tpe) = &action_success_info.target_kind {
        let action_data = user_defined_action_state.run_always.get(tpe);
        if let Some(action_data) = action_data {
            if !action_data.is_empty() {
                let mut candidate_correction_commands: Vec<CommandLineAction> = vec![];
                let error_streams = text_logs_from_success(action_success_info).await;
                extract_configured_regexes(
                    &action_success_info.label,
                    &error_streams,
                    &mut candidate_correction_commands,
                    action_data,
                );
                return apply_candidates(candidate_correction_commands, command_line_runner).await;
            }
        }
    }
    super::Response::new(Vec::default())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_regex_parsing() {
        // This was referring to a random string put into the dependencies list of the target
        let action_failed_error_info = hydrated_stream::ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            stderr: None,
            stdout: None,
            target_kind: Some(String::from("scala_library")),
        };

        let error_streams = vec![
            String::from(
                "error: Target '@foo_bar_baz//example/foo/bar:baz' is specified as __find_me__ a dependency to //example/foo/bar/baz:my_target but isn't used, please remove it from the deps.
You can use the following buildozer command:
buildozer 'remove deps @foo_bar_baz//example/foo/bar:baz' //example/foo/bar/baz:my_target"
            )
        ];

        let action_data = vec![Arc::new((
            Regex::new(r#".*specified as (.*) a dependency.*"#).unwrap(),
            ErrorProcessor {
                name: String::from("my_command"),
                active_action_type: String::from("not_tested"),
                run_on_success: true,
                regex_match: String::from("not used"),
                target_command_line: String::from("my commands: {1}"),
            },
        ))];
        let mut results = vec![];
        extract_configured_regexes(
            &action_failed_error_info.label,
            &error_streams,
            &mut results,
            &action_data,
        );
        assert_eq!(
            results,
            vec![CommandLineAction {
                config_name: "my_command".to_string(),
                label: "//src/main/com/example/foo:Bar".to_string(),
                command_line: "my commands: __find_me__".to_string(),
                why: "Triggered from user config".to_string()
            }]
        );
    }

    use crate::hydrated_stream_processors::process_bazel_failures::process_user_defined_actions::tests::ActionLogEntry::ExecuteCommandLine;
    use super::super::command_line_runner::test_tools::*;
    use crate::hydrated_stream_processors::process_bazel_failures::ExecutionResult;
    #[tokio::test]
    async fn test_apply_candidates() {
        // This was referring to a random string put into the dependencies list of the target

        let fake = FakeCommandLineRunner::default();

        let resp = apply_candidates(
            vec![CommandLineAction {
                config_name: "cfg".to_string(),
                label: "//foo/bar/baz".to_string(),
                command_line: "a b c".to_string(),
                why: "dunno".to_string(),
            }],
            fake.clone(),
        )
        .await;
        let results = fake.to_vec().await;
        assert_eq!(
            results,
            vec![ExecuteCommandLine {
                command_line: "a b c".to_string()
            }]
        );

        assert_eq!(resp.target_story_entries.len(), 1);

        let e = resp.target_story_entries[0].clone();
        assert_eq!(
            e.action,
            super::super::TargetStoryAction::RanUserAction {
                user_action_name: "cfg".to_string(),
                why: "dunno".to_string(),
                command_line: "a b c".to_string(),
                execution_result: ExecutionResult {
                    exit_success: true,
                    stdout: "".to_string(),
                    stderr: "".to_string()
                }
            }
        );
        assert_eq!(e.target, "//foo/bar/baz");
    }
}
