use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use crate::buildozer_driver::Buildozer;
use dashmap::{DashMap, DashSet};
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct ActionEventStream<T: Buildozer + Send + Sync + Clone + 'static> {
    index_input_location: Option<PathBuf>,
    index_table: Arc<RwLock<Option<index_table::IndexTable>>>,
    previous_global_seen: Arc<DashMap<String, DashSet<String>>>,
    buildozer: T,
}

impl<T> ActionEventStream<T>
where
    T: Buildozer + Send + Clone + Sync + 'static,
{
    pub fn new(index_input_location: Option<PathBuf>, buildozer: T) -> Self {
        Self {
            index_input_location: index_input_location,
            index_table: Arc::new(RwLock::new(None)),
            previous_global_seen: Arc::new(DashMap::new()),
            buildozer: buildozer,
        }
    }

    pub async fn ensure_table_loaded(self) -> () {
        let tbl = Arc::clone(&self.index_table);
        let v = tbl.read().await;
        if (*v).is_none() {
            drop(v);
            let mut w = tbl.write().await;
            match *w {
                None => {
                    let index_tbl = match &self.index_input_location {
                        Some(p) => {
                            if p.exists() {
                                let content = std::fs::read_to_string(p).unwrap();
                                index_table::parse_file(&content).unwrap()
                            } else {
                                index_table::IndexTable::new()
                            }
                        }
                        None => index_table::IndexTable::new(),
                    };
                    *w = Some(index_tbl);
                }
                Some(_) => (),
            }
            drop(w);
        }

        ()
    }

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> mpsc::Receiver<Option<u32>> {
        let (mut tx, next_rx) = mpsc::channel(4096);

        let self_d: ActionEventStream<T> = self.clone();

        tokio::spawn(async move {
            let mut done_load = false;
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await.unwrap();
                    }
                    Some(e) => {
                        if !done_load {
                            let nxt = self_d.clone();
                            nxt.ensure_table_loaded().await;
                            done_load = true;
                        }

                        let e = e.clone();
                        let mut tx = tx.clone();
                        let self_d: ActionEventStream<T> = self_d.clone();
                        tokio::spawn(async move {
                            match e {
                                hydrated_stream::HydratedInfo::ActionFailed(
                                    action_failed_error_info,
                                ) => {
                                    let tbl = Arc::clone(&self_d.index_table);
                                    let v = tbl.read().await;
                                    let arc = Arc::clone(&self_d.previous_global_seen);

                                    arc.entry(action_failed_error_info.label.clone())
                                        .or_insert(DashSet::new());
                                    let prev_data =
                                        arc.get(&action_failed_error_info.label).unwrap();

                                    let actions_completed = super::process_missing_dependency_errors::process_missing_dependency_errors(
                                            &prev_data,
                                            self_d.buildozer,
                                            &action_failed_error_info,
                                            v.as_ref().unwrap(),
                                        ).await;

                                    if actions_completed > 0 {
                                        tx.send(Some(actions_completed)).await.unwrap();
                                    }
                                }

                                hydrated_stream::HydratedInfo::BazelAbort(
                                    bazel_abort_error_info,
                                ) => {
                                    let actions_completed = super::process_build_abort_errors::process_build_abort_errors(
                                            self_d.buildozer,
                                            &bazel_abort_error_info
                                        ).await;

                                    if actions_completed > 0 {
                                        tx.send(Some(actions_completed)).await.unwrap();
                                    }
                                }
                                hydrated_stream::HydratedInfo::TargetComplete(_) => {}
                                hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
                                hydrated_stream::HydratedInfo::Progress(progress_info) => {
                                    let tbl = Arc::clone(&self_d.previous_global_seen);

                                    let actions_completed =
                                        super::process_build_abort_errors::process_progress(
                                            self_d.buildozer,
                                            &progress_info,
                                            tbl,
                                        )
                                        .await;

                                    if actions_completed > 0 {
                                        tx.send(Some(actions_completed)).await.unwrap();
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });
        next_rx
    }
}
