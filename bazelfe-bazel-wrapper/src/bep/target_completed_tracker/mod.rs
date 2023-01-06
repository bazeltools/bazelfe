use std::{collections::HashSet, sync::Arc};

use tokio::sync::Mutex;

use crate::bep::build_events::hydrated_stream;

#[derive(Clone, Debug)]
pub struct TargetCompletedTracker {
    pub expected_targets: Arc<Mutex<HashSet<String>>>,
}

#[async_trait::async_trait]
impl<T> super::BazelEventHandler<T> for TargetCompletedTracker {
    async fn process_event(
        &self,
        _bazel_run_id: usize,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<T> {
        match event {
            hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                let mut guard = self.expected_targets.lock().await;
                guard.remove(&tce.label);
            }
            _ => (),
        };
        Vec::default()
    }
}

impl TargetCompletedTracker {
    pub fn new(expected_targets: HashSet<String>) -> Self {
        Self {
            expected_targets: Arc::new(Mutex::new(expected_targets)),
        }
    }
}
