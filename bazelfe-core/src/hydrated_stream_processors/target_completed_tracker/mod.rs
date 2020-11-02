use std::{collections::HashSet, sync::Arc};

use tokio::sync::Mutex;

use crate::build_events::hydrated_stream;

#[derive(Clone, Debug)]
pub struct TargetCompletedTracker {
    pub expected_targets: Arc<Mutex<HashSet<String>>>,
}

#[async_trait::async_trait]
impl super::BazelEventHandler for TargetCompletedTracker {
    async fn process_event(
        &self,
        event: &hydrated_stream::HydratedInfo,
    ) -> Option<super::BuildEventResponse> {
        self.process(event).await
    }
}
impl TargetCompletedTracker {
    pub fn new(expected_targets: HashSet<String>) -> Self {
        Self {
            expected_targets: Arc::new(Mutex::new(expected_targets)),
        }
    }
    pub async fn process(
        &self,
        event: &hydrated_stream::HydratedInfo,
    ) -> Option<super::BuildEventResponse> {
        match event {
            hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                let mut guard = self.expected_targets.lock().await;
                guard.remove(&tce.label);
            }
            _ => (),
        };
        None
    }
}
