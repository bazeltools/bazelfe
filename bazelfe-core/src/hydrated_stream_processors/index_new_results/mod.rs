use std::path::PathBuf;

use bazelfe_protos::build_event_stream;

use crate::{build_events::hydrated_stream, index_table};

#[derive(Clone, Debug)]

pub struct Response {
    pub jvm_segments_indexed: u32,
}
impl Default for Response {
    fn default() -> Self {
        Self {
            jvm_segments_indexed: 0,
        }
    }
}
impl Response {
    pub fn new(jvm_segments_indexed: u32) -> Self {
        Self {
            jvm_segments_indexed: jvm_segments_indexed,
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexNewResults {
    index_table: index_table::IndexTable,
}

#[async_trait::async_trait]
impl super::BazelEventHandler for IndexNewResults {
    async fn process_event(
        &self,
        _bazel_run_id: usize,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        self.process(event).await
    }
}
impl IndexNewResults {
    pub fn new(index_table: index_table::IndexTable) -> Self {
        Self {
            index_table: index_table,
        }
    }
    pub async fn process(
        &self,
        event: &hydrated_stream::HydratedInfo,
    ) -> Vec<super::BuildEventResponse> {
        let r = match event {
            hydrated_stream::HydratedInfo::TargetComplete(tce) => {
                if let Some(target_kind) = &tce.target_kind {
                    if target_kind.contains("_test") {
                        return Vec::default();
                    }
                }
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
                    if let Some(of) = of.file.as_ref() {
                        if let build_event_stream::file::File::Uri(e) = of {
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
                }

                let jvm_segments_indexed = self
                    .index_table
                    .index_jar(&tce.target_kind, label, files)
                    .await;
                Some(Response::new(jvm_segments_indexed))
            }
            _ => None,
        };
        r.map(|r| super::BuildEventResponse::IndexedResults(r))
            .into_iter()
            .collect()
    }
}
