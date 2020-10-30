

use std::path::PathBuf;

use bazelfe_protos::build_event_stream;

use crate::{build_events::hydrated_stream, index_table};

#[derive(Clone, Debug)]

pub struct Response {
    pub jars_indexed: u32,
}
impl Default for Response {
    fn default() -> Self {
        Self {
            jars_indexed: 0,
        }
    }
}
impl Response {
    pub fn new(jars_indexed: u32) -> Self {
        Self {
            jars_indexed: jars_indexed
        }
    }
}

#[derive(Clone, Debug)]
pub struct IndexNewResults {
    index_table: index_table::IndexTable,
}

#[async_trait::async_trait]
impl super::BazelEventHandler for IndexNewResults {

    async fn process_event(&self, event: &hydrated_stream::HydratedInfo) -> Option<super::BuildEventResponse> {
        self.process(event).await
    }
    
}
impl IndexNewResults {
    pub fn new(index_table: index_table::IndexTable) -> Self {
        Self {
            index_table: index_table
        }
    }
pub async fn process(&self, 
    event: &hydrated_stream::HydratedInfo
) -> Option<super::BuildEventResponse> {
                        let r = match event {
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

                                self.index_table
                                    .index_jar(&tce.target_kind, label, files)
                                    .await;
                                    Some(Response::new(1))
                            },
                            _ => None
                        };
                        r.map(|r| super::BuildEventResponse::IndexedResults(r))
    }
}