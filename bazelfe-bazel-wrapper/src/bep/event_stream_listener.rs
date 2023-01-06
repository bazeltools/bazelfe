use std::sync::{atomic::AtomicUsize, Arc};

use crate::bep::build_events::hydrated_stream;

use super::BazelEventHandler;

#[derive(Debug)]
pub struct EventStreamListener<T> {
    processors: Vec<Arc<dyn BazelEventHandler<T>>>,
    run_id: Arc<AtomicUsize>,
}

impl<T> EventStreamListener<T>
where
    T: Send + 'static,
{
    pub fn new(processors: Vec<Arc<dyn BazelEventHandler<T>>>) -> Self {
        Self {
            processors,
            run_id: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn add_event_handler(&mut self, event_handler: Arc<dyn BazelEventHandler<T>>) {
        self.processors.push(event_handler);
    }

    pub fn handle_stream(
        &self,
        rx: async_channel::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> async_channel::Receiver<T> {
        let (tx, next_rx) = async_channel::unbounded();

        let current_id = self
            .run_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let processors = Arc::new(self.processors.clone());
        for _ in 0..12 {
            let rx = rx.clone();
            let tx = tx.clone();
            let processors = Arc::clone(&processors);
            tokio::spawn(async move {
                while let Ok(action) = rx.recv().await {
                    match action {
                        None => (),
                        Some(e) => {
                            for p in processors.iter() {
                                for r in p.process_event(current_id, &e).await.into_iter() {
                                    tx.send(r).await.unwrap();
                                }
                            }
                        }
                    }
                }
            });
        }
        next_rx
    }
}
