use crate::build_events::hydrated_stream;

pub mod process_bazel_failures;
pub mod index_new_results;
pub mod event_stream_listener;

#[derive(Clone, Debug)]
pub enum BuildEventResponse {
    ProcessedBuildFailures(process_bazel_failures::Response),
    IndexedResults(index_new_results::Response)
}
#[async_trait::async_trait]
pub trait BazelEventHandler: std::fmt::Debug + Send + Sync {
    async fn process_event(&self, event: &hydrated_stream::HydratedInfo) -> Option<BuildEventResponse>;
}