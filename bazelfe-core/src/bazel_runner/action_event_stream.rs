use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use crate::buildozer_driver::Buildozer;
use bazelfe_protos::*;
use dashmap::{DashMap, DashSet};

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct ActionEventStream<T: Buildozer + Send + Sync + Clone + 'static> {
    index_table: index_table::IndexTable,
    previous_global_seen: Arc<DashMap<String, DashSet<String>>>,
    buildozer: T,
}

impl<T> ActionEventStream<T>
where
    T: Buildozer + Send + Clone + Sync + 'static,
{
    pub fn new(index_table: index_table::IndexTable, buildozer: T) -> Self {
        Self {
            index_table: index_table,
            previous_global_seen: Arc::new(DashMap::new()),
            buildozer: buildozer,
        }
    }

    pub fn build_action_pipeline(
        &self,
        rx: async_channel::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> async_channel::Receiver<Option<u32>> {
        let (tx, next_rx) = async_channel::unbounded();

        let self_d: ActionEventStream<T> = self.clone();

        for _ in 0..12 {
            let rx = rx.clone();
            let tx = tx.clone();
            let self_d: ActionEventStream<T> = self_d.clone();

            tokio::spawn(async move {
                while let Ok(action) = rx.recv().await {
                    match action {
                        None => {
                            tx.send(None).await.unwrap();
                        }
                        Some(e) => {
                            let e = e.clone();
                            let tx = tx.clone();
                            let self_d: ActionEventStream<T> = self_d.clone();
                            tokio::spawn(async move {
                                match e {
                                    hydrated_stream::HydratedInfo::ActionFailed(
                                        action_failed_error_info,
                                    ) => {
                                        let arc = Arc::clone(&self_d.previous_global_seen);

                                        arc.entry(action_failed_error_info.label.clone())
                                            .or_insert(DashSet::new());
                                        let prev_data =
                                            arc.get(&action_failed_error_info.label).unwrap();

                                        let actions_completed = super::process_missing_dependency_errors::process_missing_dependency_errors(
                                            &prev_data,
                                            self_d.buildozer,
                                            &action_failed_error_info,
                                            &self_d.index_table,
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
                                    hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                                        let mut files = Vec::default();
                                        for of in tce.output_files.iter() {
                                            if let build_event_stream::file::File::Uri(e) = of {
                                                if e.ends_with(".jar") && e.starts_with("file://") {
                                                    let u: PathBuf =
                                                        e.strip_prefix("file://").unwrap().into();
                                                    files.push(u);
                                                }
                                            }
                                        }

                                        self_d
                                            .index_table
                                            .index_jar(tce.label.clone(), files)
                                            .await;
                                    }
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
        }
        next_rx
    }
}
