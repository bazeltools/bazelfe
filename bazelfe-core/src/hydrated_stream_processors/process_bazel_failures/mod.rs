use std::{sync::Arc, time::Instant};

use dashmap::{DashMap, DashSet};

use crate::{buildozer_driver::Buildozer, build_events::hydrated_stream, index_table};

mod process_missing_dependency_errors;
mod process_build_abort_errors;

#[derive(Clone, Debug)]
pub enum TargetStoryAction {
    AddedDependency {
        added_what: String,
        why: String
    },
    RemovedDependency {
        removed_what: String,
        why: String
    },
    Success
}
#[derive(Clone, Debug)]
pub struct TargetStory {
    pub target: String,
    pub action: TargetStoryAction,
    pub when: Instant
}

#[derive(Clone, Debug)]
pub struct Response {
    pub actions_completed: u32,
    pub target_story_entries: Vec<TargetStory>
}
impl Response {
    pub fn new(actions_completed: u32, target_story_entries: Vec<TargetStory>) -> Self {
        Self {
            actions_completed: actions_completed,
            target_story_entries: target_story_entries
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessBazelFailures<T: Buildozer> {
    index_table: index_table::IndexTable,
    previous_global_seen: Arc<DashMap<String, DashSet<String>>>,
    buildozer: T,
}


#[async_trait::async_trait]
impl<T: Buildozer> super::BazelEventHandler for ProcessBazelFailures<T> {

    async fn process_event(&self, event: &hydrated_stream::HydratedInfo) -> Option<super::BuildEventResponse> {
        self.process(event).await
    }
    
}
impl<T: Buildozer> ProcessBazelFailures<T> {
    pub fn new(index_table:index_table::IndexTable, buildozer: T) -> Self {
        Self {
            previous_global_seen: Arc::new(DashMap::new()),
            index_table: index_table,
            buildozer: buildozer
        }
    }
pub async fn process(&self, 
    event: &hydrated_stream::HydratedInfo
) -> Option<super::BuildEventResponse> {
                        let r = match event {
                            hydrated_stream::HydratedInfo::ActionFailed(
                                action_failed_error_info,
                            ) => {
                                let arc = Arc::clone(&self.previous_global_seen);

                                arc.entry(action_failed_error_info.label.clone())
                                    .or_insert(DashSet::new());
                                let prev_data =
                                    arc.get(&action_failed_error_info.label).unwrap();
                                Some(process_missing_dependency_errors::process_missing_dependency_errors(
                                        &prev_data,
                                        self.buildozer.clone(),
                                        &action_failed_error_info,
                                        &self.index_table,
                                    ).await)
                            }

                            hydrated_stream::HydratedInfo::BazelAbort(
                                bazel_abort_error_info,
                            ) => {
                                Some(process_build_abort_errors::process_build_abort_errors(
                                        self.buildozer.clone(),
                                        &bazel_abort_error_info
                                    ).await)

                            }
                            hydrated_stream::HydratedInfo::TargetComplete(_) => None,
                            hydrated_stream::HydratedInfo::ActionSuccess(action_success) => {
                                Some(Response::new(0, vec![
                                    TargetStory{
                                        target: action_success.label.clone(),
                                        action: TargetStoryAction::Success,
                                        when: Instant::now()
                                    }
                                ]))
                            }
                            hydrated_stream::HydratedInfo::Progress(progress_info) => {
                                let tbl = Arc::clone(&self.previous_global_seen);

                                Some(
                                    process_build_abort_errors::process_progress(
                                        self.buildozer.clone(),
                                        &progress_info,
                                        tbl,
                                    )
                                    .await)

                            }
                        };
                        r.and_then(|r| {
                            if r.actions_completed > 0 || r.target_story_entries.len() > 0 {
                                Some(super::BuildEventResponse::ProcessedBuildFailures(r))
                            } else {
                                None
                            }
                        })
                        
    }
}