use std::{collections::HashSet, path::PathBuf};

use bazelfe_bazel_wrapper::bep::BazelEventHandler;
use bazelfe_protos::build_event_stream;
use build_event_stream::file::File::Uri;

use crate::{config::IndexerConfig, index_table};

use super::BuildEventResponse;

#[derive(Clone, Debug, Default)]
pub struct Response {
    pub jvm_segments_indexed: u32,
}

impl Response {
    pub fn new(jvm_segments_indexed: u32) -> Self {
        Self {
            jvm_segments_indexed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexNewResults {
    index_table: index_table::IndexTable,
    blacklist_target_kind: HashSet<String>,
}

#[async_trait::async_trait]
impl BazelEventHandler<BuildEventResponse> for IndexNewResults {
    async fn process_event(
        &self,
        _bazel_run_id: usize,
        event: &bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        self.process(event).await
    }
}
impl IndexNewResults {
    pub fn new(index_table: index_table::IndexTable, indexer_config: &IndexerConfig) -> Self {
        let mut blacklist_target_kind: HashSet<String> = Default::default();

        for t in indexer_config.blacklist_rule_kind.iter() {
            blacklist_target_kind.insert(t.clone());
        }
        Self {
            index_table,
            blacklist_target_kind,
        }
    }
    pub async fn process(
        &self,
        event: &bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        let r = match event {
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                if let Some(target_kind) = &tce.target_kind {
                    if (target_kind.contains("_test")
                        || (target_kind.contains("generated file")
                            && tce.label.ends_with("_deploy.jar")))
                        || self.blacklist_target_kind.contains(target_kind)
                    {
                        // Deploy jar, test thing or just in a blacklist of rules
                        return Vec::default();
                    }
                }
                let label = tce.label.clone();
                let mut files = Vec::default();

                let external_match = if label.starts_with('@') {
                    let idx = label.find('/').unwrap();
                    let repo = &label[1..idx];
                    let path_segment = format!("external/{}", repo);
                    Some(path_segment)
                } else {
                    None
                };

                for of in tce.output_files.iter() {
                    if let Some(Uri(e)) = of.file.as_ref() {
                        if e.ends_with(".jar") && e.starts_with("file://") {
                            let a = e.strip_prefix("file://").unwrap();
                            let allowed = if let Some(ref external_repo) = external_match {
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

                let jvm_segments_indexed = self
                    .index_table
                    .index_jar(&tce.target_kind, label, files)
                    .await;
                Some(Response::new(jvm_segments_indexed))
            }
            _ => None,
        };
        r.map(super::BuildEventResponse::IndexedResults)
            .into_iter()
            .collect()
    }
}
