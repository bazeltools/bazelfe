use crate::build_events::hydrated_stream;

pub mod event_stream_listener;
pub mod index_new_results;
pub mod process_bazel_failures;
pub mod target_completed_tracker;

#[derive(Clone, Debug)]
pub enum BuildEventResponse {
    ProcessedBuildFailures(process_bazel_failures::Response),
    IndexedResults(index_new_results::Response),
}
#[async_trait::async_trait]
pub trait BazelEventHandler: std::fmt::Debug + Send + Sync {
    async fn process_event(
        &self,
        bazel_run_id: usize,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<BuildEventResponse>;
}
