use std::{collections::HashMap, collections::HashSet, sync::Arc, time::Instant};

use tokio::sync::{Mutex, RwLock};

use crate::{
    build_events::hydrated_stream, buildozer_driver::Buildozer, config::Config, index_table,
};

use self::{
    command_line_runner::ExecutionResult,
    process_user_defined_actions::UserDefinedActionsStateCache,
};

mod command_line_runner;
mod process_action_failure_error;
mod process_build_abort_errors;
mod process_missing_dependency_errors;
mod process_user_defined_actions;
mod shared_utils;

pub use command_line_runner::CommandLineRunner;
pub use command_line_runner::CommandLineRunnerImpl;

#[derive(Clone, Debug, PartialEq)]
pub enum TargetStoryAction {
    AddedDependency {
        added_what: String,
        why: String,
    },
    RemovedDependency {
        removed_what: String,
        why: String,
    },
    RanUserAction {
        user_action_name: String,
        why: String,
        command_line: String,
        execution_result: ExecutionResult,
    },
    Success,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TargetStory {
    pub target: String,
    pub action: TargetStoryAction,
    pub when: Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    pub target_story_entries: Vec<TargetStory>,
}
impl Response {
    pub fn new(target_story_entries: Vec<TargetStory>) -> Self {
        Self {
            target_story_entries: target_story_entries,
        }
    }
}

#[derive(Debug)]
pub struct CurrentState {
    pub ignore_list: HashSet<String>,
    pub added_target_for_class: HashMap<crate::error_extraction::ActionRequest, HashSet<String>>,
    pub epoch: usize,
}
impl Default for CurrentState {
    fn default() -> Self {
        Self {
            ignore_list: HashSet::default(),
            added_target_for_class: HashMap::default(),
            epoch: 0,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ProcessBazelFailures<T: Buildozer, U: CommandLineRunner> {
    index_table: index_table::IndexTable,
    previous_global_seen: Arc<RwLock<HashMap<String, Arc<Mutex<CurrentState>>>>>,
    epoch: Arc<RwLock<usize>>,
    buildozer: T,
    command_line_runner: U,
    config: Arc<Config>,
    user_defined_action_cache: Arc<UserDefinedActionsStateCache>,
}

#[async_trait::async_trait]
impl<T: Buildozer, U: CommandLineRunner> super::BazelEventHandler for ProcessBazelFailures<T, U> {
    async fn process_event(
        &self,
        _bazel_run_id: usize,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        self.process(event).await
    }
}
impl<T: Buildozer, U: CommandLineRunner> ProcessBazelFailures<T, U> {
    pub fn new(
        index_table: index_table::IndexTable,
        buildozer: T,
        command_line_runner: U,
        config: Arc<Config>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let user_defined_action_cache =
            Arc::new(UserDefinedActionsStateCache::from_config(&config)?);
        Ok(Self {
            previous_global_seen: Arc::new(RwLock::new(HashMap::default())),
            index_table: index_table,
            buildozer: buildozer,
            command_line_runner,
            epoch: Arc::new(RwLock::new(0)),
            config,
            user_defined_action_cache,
        })
    }

    pub async fn advance_epoch(&self) -> () {
        let mut e = self.epoch.write().await;
        *e += 1;
    }
    pub async fn process(
        &self,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        let r: Vec<Response> = match event {
            hydrated_stream::HydratedInfo::ActionFailed(action_failed_error_info) => {
                let arc = Arc::clone(&self.previous_global_seen);

                let prev_data_arc = {
                    let handle = self.previous_global_seen.read().await;
                    match handle.get(&action_failed_error_info.label) {
                        Some(e) => Arc::clone(e),
                        None => {
                            drop(handle);
                            let mut handle = arc.write().await;
                            handle
                                .insert(action_failed_error_info.label.clone(), Default::default());
                            drop(handle);
                            let handle = arc.read().await;
                            Arc::clone(handle.get(&action_failed_error_info.label).unwrap())
                        }
                    }
                };
                let mut prev_data = prev_data_arc.lock().await;
                let epoch = self.epoch.read().await.clone();

                let action_failed_response = process_action_failure_error::process_action_failed(
                    self.buildozer.clone(),
                    &action_failed_error_info,
                )
                .await;

                let missing_dependencies_response =
                    process_missing_dependency_errors::process_missing_dependency_errors(
                        &mut *prev_data,
                        self.buildozer.clone(),
                        &action_failed_error_info,
                        &self.index_table,
                        epoch,
                    )
                    .await;

                let user_defined_action_failure =
                    process_user_defined_actions::process_action_failed(
                        self.command_line_runner.clone(),
                        &action_failed_error_info,
                        &self.user_defined_action_cache,
                    )
                    .await;

                vec![
                    action_failed_response,
                    missing_dependencies_response,
                    user_defined_action_failure,
                ]
            }

            hydrated_stream::HydratedInfo::BazelAbort(bazel_abort_error_info) => vec![
                process_build_abort_errors::process_build_abort_errors(
                    self.buildozer.clone(),
                    &bazel_abort_error_info,
                )
                .await,
            ],
            hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                if tce.success && tce.label.len() > 0 {
                    vec![Response::new(vec![TargetStory {
                        target: tce.label.clone(),
                        action: TargetStoryAction::Success,
                        when: Instant::now(),
                    }])]
                } else {
                    Vec::default()
                }
            }
            // action successes can a be a bit hard to use since a rule often has ~several actions
            // things like writing out the input for a compiler step is of course its own action too.
            hydrated_stream::HydratedInfo::ActionSuccess(action_success_info) => {
                let action_success_response = process_user_defined_actions::process_action_success(
                    self.command_line_runner.clone(),
                    &action_success_info,
                    &self.user_defined_action_cache,
                )
                .await;

                vec![action_success_response]
            }
            hydrated_stream::HydratedInfo::Progress(progress_info) => {
                let tbl = Arc::clone(&self.previous_global_seen);

                vec![
                    process_build_abort_errors::process_progress(
                        self.buildozer.clone(),
                        &progress_info,
                        tbl,
                    )
                    .await,
                ]
            }
            hydrated_stream::HydratedInfo::TestResult(_) => {
                vec![]
            }
        };
        r.into_iter()
            .filter_map(|e| {
                if e.target_story_entries.len() > 0 {
                    Some(super::BuildEventResponse::ProcessedBuildFailures(e))
                } else {
                    None
                }
            })
            .collect()
    }
}
