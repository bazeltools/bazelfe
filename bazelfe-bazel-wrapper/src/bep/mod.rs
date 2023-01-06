pub mod build_events;
pub mod target_completed_tracker;

use crate::bep::build_events::hydrated_stream;

mod event_stream_listener;
pub use event_stream_listener::EventStreamListener;

#[async_trait::async_trait]
pub trait BazelEventHandler<T>: std::fmt::Debug + Send + Sync {
    async fn process_event(
        &self,
        bazel_run_id: usize,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<T>;
}
