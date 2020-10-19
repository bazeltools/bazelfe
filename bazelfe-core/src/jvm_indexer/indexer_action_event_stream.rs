use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use bazelfe_protos::*;
use dashmap::DashMap;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct IndexerActionEventStream {
    index_table: Arc<RwLock<index_table::IndexTable>>,
    allowed_rule_kinds: Arc<HashSet<String>>,
}

impl IndexerActionEventStream {
    pub fn new(allowed_rule_kinds: Vec<String>) -> Self {
        let mut allowed = HashSet::new();
        for e in allowed_rule_kinds.into_iter() {
            allowed.insert(e);
        }
        Self {
            index_table: Arc::new(RwLock::new(index_table::IndexTable::default())),
            allowed_rule_kinds: Arc::new(allowed),
        }
    }

    pub fn build_action_pipeline(
        &self,
        mut rx: mpsc::Receiver<Option<hydrated_stream::HydratedInfo>>,
        results_map: Arc<DashMap<String, Vec<String>>>,
    ) -> mpsc::Receiver<Option<usize>> {
        let (mut tx, next_rx) = mpsc::channel(4096);

        let allowed_rule_kind = Arc::clone(&self.allowed_rule_kinds);

        tokio::spawn(async move {
            while let Some(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await.unwrap();
                    }
                    Some(e) => {
                        let e = e.clone();
                        let allowed_rule_kind = Arc::clone(&allowed_rule_kind);
                        let mut tx = tx.clone();
                        let results_map = Arc::clone(&results_map);
                        tokio::spawn(async move {
                            match e {
                                hydrated_stream::HydratedInfo::ActionFailed(
                                    _,
                                ) => {
                                  
                                }
                                hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
                                hydrated_stream::HydratedInfo::BazelAbort(_) => {
                                    // aborts can/will occur when we loop through things if stuff depends on an external target
                                    // we don't have configured
                                }
                                hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                                    if let Some(ref target_kind) = tce.target_kind {
                                        if allowed_rule_kind.contains(target_kind) {
                                            let mut found_classes = Vec::default();

                                            for of in tce.output_files.iter() {
                                                if let build_event_stream::file::File::Uri(e) = of {
                                                    if e.starts_with("file://") {
                                                        let u: PathBuf = e
                                                            .strip_prefix("file://")
                                                            .unwrap()
                                                            .into();
                                                        let extracted_zip = crate::zip_parse::extract_classes_from_zip(u);
                                                        found_classes.extend(
                                                            transform_file_names_into_class_names(
                                                                extracted_zip,
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                            tx.send(Some(found_classes.len())).await.unwrap();
                                            found_classes.sort();
                                            found_classes.dedup();
                                            results_map.insert(tce.label, found_classes);
                                        }
                                    }
                                }

                                hydrated_stream::HydratedInfo::Progress(_) => {
                                    
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_file_name_to_class_name() {
        let sample_inputs = vec![
            "scala/reflect/internal/SymbolPairs$Cursor$$anon$1.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anon$2.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anonfun$init$2$$anonfun$apply$1.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anonfun$init$2$$anonfun$apply$2.class",
            "scala/reflect/internal/ReificationSupport$ReificationSupportImpl$UnMkTemplate$$anonfun$ctorArgsCorrespondToFields$1$1.class",
            "scala/reflect/internal/Depth$.class",
            "scala/reflect/internal/Depth.class",
            "com/android/aapt/Resources$AllowNew$1.class",
            "com/android/aapt/Resources$AllowNew$Builder.class",
            "com/android/aapt/Resources$AllowNew.class",
            "com/android/aapt/Resources$AllowNewOrBuilder.class",
        ];

        let expected_results: Vec<String> = vec![
            "com.android.aapt.Resources.AllowNew",
            "com.android.aapt.Resources.AllowNew.Builder",
            "com.android.aapt.Resources.AllowNewOrBuilder",
            "scala.reflect.internal.Depth",
            "scala.reflect.internal.ReificationSupport.ReificationSupportImpl.UnMkTemplate",
            "scala.reflect.internal.SymbolPairs.Cursor",
        ]
        .into_iter()
        .map(|e| e.to_string())
        .collect();

        assert_eq!(
            transform_file_names_into_class_names(
                sample_inputs.into_iter().map(|e| e.to_string()).collect()
            ),
            expected_results
        );
    }
}
