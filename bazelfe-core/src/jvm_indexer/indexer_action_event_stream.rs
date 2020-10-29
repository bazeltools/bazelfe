use std::{path::PathBuf, sync::Arc};

use crate::build_events::hydrated_stream;

use super::super::index_table;
use bazelfe_protos::*;
use dashmap::DashMap;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;
use tokio::sync::RwLock;

pub trait ExtractClassData<U> {
    fn paths(&self) -> Vec<PathBuf>;
    fn id_info(&self) -> U;
}
#[derive(Clone, Debug)]
pub struct IndexerActionEventStream {
    pub index_table: index_table::IndexTable,
    allowed_rule_kinds: Arc<HashSet<String>>,
}

impl IndexerActionEventStream {
    pub fn new(allowed_rule_kinds: Vec<String>) -> Self {
        let mut allowed = HashSet::new();
        for e in allowed_rule_kinds.into_iter() {
            allowed.insert(e);
        }
        Self {
            index_table: index_table::IndexTable::default(),
            allowed_rule_kinds: Arc::new(allowed),
        }
    }

    pub fn build_action_pipeline(
        &self,
        rx: async_channel::Receiver<Option<hydrated_stream::HydratedInfo>>,
    ) -> async_channel::Receiver<Option<usize>> {
        let (tx, next_rx) = async_channel::unbounded();

        let self_d = self.clone();
        tokio::spawn(async move {
            while let Ok(action) = rx.recv().await {
                match action {
                    None => {
                        tx.send(None).await.unwrap();
                    }
                    Some(e) => {
                        let e = e.clone();
                        let self_d = self_d.clone();

                        tokio::spawn(async move {
                            match e {
                                hydrated_stream::HydratedInfo::ActionFailed(_) => {}
                                hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
                                hydrated_stream::HydratedInfo::BazelAbort(_) => {
                                    // aborts can/will occur when we loop through things if stuff depends on an external target
                                    // we don't have configured
                                }
                                hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                                    let label = tce.label.clone();
                                    let mut files = Vec::default();

                                    let external_match = if label.starts_with("@") {
                                        let idx = label.find('/').unwrap();
                                        let repo = &label[1..idx];
                                        let path_segment = format!("external/{}", repo);
                                        Some(path_segment)
                                    } else {
                                        None
                                    };

                                    for of in tce.output_files.iter() {
                                        if let build_event_stream::file::File::Uri(e) = of {
                                            if e.ends_with(".jar") && e.starts_with("file://") {
                                                let a = e.strip_prefix("file://").unwrap();
                                                let allowed = if let Some(ref external_repo) =
                                                    external_match
                                                {
                                                    a.contains(external_repo)
                                                } else {
                                                    !a.contains("/external/")
                                                };
                                                if allowed {
                                                    let u: PathBuf = a.into();
                                                    files.push(u);
                                                }
                                            }
                                        }
                                    }

                                    self_d.index_table.index_jar(label, files).await;
                                }

                                hydrated_stream::HydratedInfo::Progress(_) => {}
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
