use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use crate::buildozer_driver::Buildozer;
use bazelfe_protos::*;
use dashmap::{DashMap, DashSet};
use lazy_static::lazy_static;
use regex::Regex;
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

fn remove_from<'a>(haystack: &'a str, needle: &str) -> &'a str {
    match haystack.find(needle) {
        None => haystack,
        Some(pos) => &haystack[0..pos],
    }
}
fn transform_file_names_into_class_names(class_names: Vec<String>) -> Vec<String> {
    lazy_static! {
        static ref SUFFIX_ANON_CLAZZES: Regex = Regex::new(r"(\$\d*)?\.class$").unwrap();
    }

    let mut vec: Vec<String> = class_names
        .into_iter()
        .filter_map(|e| {
            if e.ends_with(".class") {
                Some(remove_from(&SUFFIX_ANON_CLAZZES.replace(&e, ""), "$$").to_string())
            } else {
                None
            }
        })
        .map(|e| e.replace("$", ".").replace("/", "."))
        .collect();
    vec.sort();
    vec.dedup();
    vec
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
        rx: async_channel::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> async_channel::Receiver<Option<u32>> {
        let (tx, next_rx) = async_channel::unbounded();

        let self_d: ActionEventStream<T> = self.clone();

        for idx in 0..12 {
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
                            let mut tx = tx.clone();
                            let self_d: ActionEventStream<T> = self_d.clone();
                            tokio::spawn(async move {
                                match e {
                                    hydrated_stream::HydratedInfo::ActionFailed(
                                        action_failed_error_info,
                                    ) => {
                                        self.ensure_table_loaded().await;

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
                                    hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                                        self.ensure_table_loaded().await;
                                        let mut found_classes = Vec::default();

                                        for of in tce.output_files.iter() {
                                            if let build_event_stream::file::File::Uri(e) = of {
                                                if e.ends_with(".jar") && e.starts_with("file://") {
                                                    let u: PathBuf =
                                                        e.strip_prefix("file://").unwrap().into();
                                                    let extracted_zip =
                                                        crate::zip_parse::extract_classes_from_zip(
                                                            u,
                                                        );
                                                    found_classes.extend(
                                                        transform_file_names_into_class_names(
                                                            extracted_zip,
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                        found_classes.sort();
                                        found_classes.dedup();
                                        for clazz in found_classes.into_iter() {
                                            self.index_table.
                                            (&self, key: S, value: (u16, String)) 
                                        }
                                        println!("{:#?}", found_classes);
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
