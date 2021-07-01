use lazy_static::lazy_static;

use crate::{build_events::hydrated_stream, buildozer_driver::Buildozer};
use regex::Regex;
use std::time::Instant;

use super::shared_utils::text_logs_from_failure;

#[derive(Clone, PartialEq, Debug)]

enum BazelCorrectionCommand {
    BuildozerRemoveDep(BuildozerRemoveDepCmd),
}
#[derive(Clone, PartialEq, Debug)]
struct BuildozerRemoveDepCmd {
    pub target_to_operate_on: String,
    pub dependency_to_remove: String,
    pub why: String,
}

fn extract_dependency_isnt_used(
    _action_failed_error_info: &hydrated_stream::ActionFailedErrorInfo,
    input_error_streams: &Vec<String>,
    command_stream: &mut Vec<BazelCorrectionCommand>,
) {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^\s*error: Target '(.*)' is specified as a dependency to ([^ ]*) but isn't used, please remove it from the deps.\s*$"
        )
        .unwrap();
    }

    for stream in input_error_streams {
        for ln in stream.lines() {
            let captures = RE.captures(ln);

            match captures {
                None => (),
                Some(captures) => {
                    let offending_dependency = captures.get(1).unwrap().as_str();
                    let src_target = captures.get(2).unwrap().as_str();

                    let correction =
                        BazelCorrectionCommand::BuildozerRemoveDep(BuildozerRemoveDepCmd {
                            target_to_operate_on: src_target.to_string(),
                            dependency_to_remove: offending_dependency.to_string(),
                            why: String::from("Dependency is unused"),
                        });
                    command_stream.push(correction);
                }
            }
        }
    }
}

async fn apply_candidates<T: Buildozer + Clone + Send + Sync + 'static>(
    candidate_correction_commands: Vec<BazelCorrectionCommand>,
    buildozer: T,
) -> super::Response {
    let mut target_stories = Vec::default();
    if candidate_correction_commands.len() == 0 {
        return super::Response::new(Vec::default());
    }
    for correction_command in candidate_correction_commands.into_iter() {
        match correction_command {
            BazelCorrectionCommand::BuildozerRemoveDep(buildozer_remove_dep) => {
                let dependency_to_remove = buildozer_remove_dep.dependency_to_remove;
                let target_to_operate_on = buildozer_remove_dep.target_to_operate_on;
                // otherwise... add the dependency with buildozer here
                // then add it ot the local seen dependencies
                debug!(
                    "Buildozer action: remove dependency {:?} from {:?}",
                    dependency_to_remove, target_to_operate_on
                );
                let buildozer_res = buildozer
                    .remove_dependency(&target_to_operate_on, &dependency_to_remove)
                    .await;
                match buildozer_res {
                    Ok(_) => {
                        target_stories.push(super::TargetStory {
                            target: target_to_operate_on.clone(),
                            action: super::TargetStoryAction::RemovedDependency {
                                removed_what: dependency_to_remove.clone(),
                                why: buildozer_remove_dep.why.clone(),
                            },
                            when: Instant::now(),
                        });
                    }
                    Err(_) => warn!("Buildozer command failed"),
                }
            }
        }
    }
    super::Response::new(target_stories)
}

pub async fn process_action_failed<T: Buildozer + Clone + Send + Sync + 'static>(
    buildozer: T,
    action_failed_error_info: &hydrated_stream::ActionFailedErrorInfo,
) -> super::Response {
    let mut candidate_correction_commands: Vec<BazelCorrectionCommand> = vec![];

    let error_streams = text_logs_from_failure(action_failed_error_info).await;
    extract_dependency_isnt_used(
        action_failed_error_info,
        &error_streams,
        &mut candidate_correction_commands,
    );
    apply_candidates(candidate_correction_commands, buildozer).await
}

#[cfg(test)]
mod tests {

    use super::*;
    use hydrated_stream::ActionFailedErrorInfo;
    #[test]
    fn test_extract_dependency_isnt_used() {
        // This was referring to a random string put into the dependencies list of the target
        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            target_kind: Some(String::from("scala_library")),
            stdout: None,
            stderr: None,
        };

        let error_streams = vec![
            String::from(
                "error: Target '@foo_bar_baz//example/foo/bar:baz' is specified as a dependency to //example/foo/bar/baz:my_target but isn't used, please remove it from the deps.
You can use the following buildozer command:
buildozer 'remove deps @foo_bar_baz//example/foo/bar:baz' //example/foo/bar/baz:my_target"
            )
        ];
        let mut results = vec![];
        extract_dependency_isnt_used(&action_failed_error_info, &error_streams, &mut results);
        assert_eq!(
            results,
            vec![BazelCorrectionCommand::BuildozerRemoveDep(
                BuildozerRemoveDepCmd {
                    target_to_operate_on: String::from("//example/foo/bar/baz:my_target"),
                    dependency_to_remove: String::from("@foo_bar_baz//example/foo/bar:baz"),
                    why: String::from("Dependency is unused"),
                }
            )]
        );
    }
}
