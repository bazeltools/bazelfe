use std::{path::PathBuf, sync::Arc};

use crate::{hydrated_stream_processors::BuildEventResponse, build_events::hydrated_stream};


#[derive(Debug)]
pub struct ActionEventStream {
    processors: Arc<Vec<Box<dyn crate::hydrated_stream_processors::BazelEventHandler>>>,
}

impl ActionEventStream
{
    pub fn new(processors: Vec<Box<dyn crate::hydrated_stream_processors::BazelEventHandler>>) -> Self {
        Self {
            processors: Arc::new(processors),
        }
    }

    pub fn build_action_pipeline(
        &self,
        rx: async_channel::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> async_channel::Receiver<BuildEventResponse> {
        let (tx, next_rx) = async_channel::unbounded();

        for _ in 0..12 {
            let rx = rx.clone();
            let tx = tx.clone();
            let processors = Arc::clone(&self.processors);
            tokio::spawn(async move {
                while let Ok(action) = rx.recv().await {

                    match action {
                        None => (),
                        Some(e) => {
                            for p in processors.iter() {
                                if let Some(r) = p.process_event(&e).await {
                                    tx.send(r).await.unwrap();
                                };
                            }
                        }
                    }
                }
            });
        }
        next_rx
    }
}
