// Not entirely sure one would want to keep these layers/separation long term
// right now this separation in writing this makes it easy to catalog the function
// and ensure its tested right.

// maps over the action stream and provides a new stream of just ErrorInfo outputs
// Unknown if we should consume this as a stream and try action failures immediately
// or wait till the operation is done not to mutate things under bazel?

use std::collections::HashMap;

use super::build_event_server::bazel_event::{self, TestResultEvt};
use super::build_event_server::BuildEventAction;
use bazelfe_protos::build_event_stream::NamedSetOfFiles;
use bazelfe_protos::*;
use std::path::PathBuf;

pub trait HasFiles {
    fn files(&self) -> Vec<build_event_stream::File>;
    fn uri_or_contents(&self) -> Vec<build_event_stream::file::File> {
        self.files().into_iter().filter_map(|e| e.file).collect()
    }
    fn path_bufs(&self) -> Vec<PathBuf> {
        self.uri_or_contents()
            .into_iter()
            .flat_map(|e| match e {
                build_event_stream::file::File::Uri(e) => {
                    if e.starts_with("file://") {
                        let u: PathBuf = e.strip_prefix("file://").unwrap().into();
                        Some(u)
                    } else {
                        log::warn!("Path isn't a file, so skipping...{:?}", e);

                        None
                    }
                }
                build_event_stream::file::File::Contents(_) => None,
            })
            .collect()
    }
}
// This is keeping some state as we go through a stream to hydrate values with things like rule kinds
// not on the indvidual events.

#[derive(Clone, PartialEq, Debug)]
pub struct ActionFailedErrorInfo {
    pub label: String,
    pub stdout: Option<build_event_stream::File>,
    pub stderr: Option<build_event_stream::File>,
    pub target_kind: Option<String>,
}
impl HasFiles for ActionFailedErrorInfo {
    fn files(&self) -> Vec<build_event_stream::File> {
        let mut r = Vec::default();

        if let Some(s) = self.stdout.as_ref() {
            r.push(s.clone());
        }

        if let Some(s) = self.stderr.as_ref() {
            r.push(s.clone());
        }
        r
    }
}
#[derive(Clone, PartialEq, Debug)]
pub struct TestResultInfo {
    pub test_summary_event: TestResultEvt,
    pub target_kind: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BazelAbortErrorInfo {
    pub label: Option<String>,
    pub reason: Option<build_event_stream::aborted::AbortReason>,
    pub description: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ActionSuccessInfo {
    pub label: String,
    pub stdout: Option<build_event_stream::File>,
    pub stderr: Option<build_event_stream::File>,
    pub target_kind: Option<String>,
}

impl HasFiles for ActionSuccessInfo {
    fn files(&self) -> Vec<build_event_stream::File> {
        let mut r = Vec::default();

        if let Some(s) = self.stdout.as_ref() {
            r.push(s.clone());
        }

        if let Some(s) = self.stderr.as_ref() {
            r.push(s.clone());
        }
        r
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TargetCompleteInfo {
    pub label: String,
    pub aspect: Option<String>,
    pub success: bool,
    pub target_kind: Option<String>,
    pub output_files: Vec<build_event_stream::File>,
}

// Broad strokes of the failure occured inside an action (most common)
// or at a bazel abort, things like mis-configured build files
#[derive(Clone, PartialEq, Debug)]
pub enum HydratedInfo {
    BazelAbort(BazelAbortErrorInfo),
    ActionFailed(ActionFailedErrorInfo),
    Progress(bazel_event::ProgressEvt),
    TestResult(TestResultInfo),
    ActionSuccess(ActionSuccessInfo),
    TargetComplete(TargetCompleteInfo),
}

fn recursive_lookup(
    lut: &HashMap<String, build_event_stream::NamedSetOfFiles>,
    results: &mut Vec<build_event_stream::File>,
    mut ids: Vec<String>,
) -> bool {
    while !ids.is_empty() {
        if let Some(head) = ids.pop() {
            if let Some(r) = lut.get(&head) {
                results.extend(r.files.iter().cloned());
                ids.extend(r.file_sets.iter().map(|e| e.id.clone()));
            } else {
                return false;
            }
        }
    }
    true
}

fn tce_event(
    tce: bazel_event::TargetCompletedEvt,
    rule_kind_lookup: &HashMap<String, String>,
    named_set_of_files_lookup: &HashMap<String, build_event_stream::NamedSetOfFiles>,
    to_revisit: &mut Vec<bazel_event::TargetCompletedEvt>,
) -> Option<TargetCompleteInfo> {
    let mut output_files = Vec::default();
    let found_everything =
        if let Some(output_grp) = &tce.output_groups.iter().find(|grp| grp.name == "default") {
            recursive_lookup(
                named_set_of_files_lookup,
                &mut output_files,
                output_grp
                    .file_sets
                    .iter()
                    .map(|fs| fs.id.clone())
                    .collect(),
            )
        } else {
            true
        };

    if found_everything {
        let target_complete_info = TargetCompleteInfo {
            output_files,
            target_kind: rule_kind_lookup.get(&tce.label).cloned(),
            aspect: tce.aspect,
            label: tce.label,
            success: tce.success,
        };
        Some(target_complete_info)
    } else {
        to_revisit.push(tce);
        None
    }
}

#[derive(Default, Debug)]
pub struct HydratorState {
    named_set_of_files_lookup: HashMap<String, NamedSetOfFiles>,
    rule_kind_lookup: HashMap<String, String>,
    buffered_tce: Vec<bazel_event::TargetCompletedEvt>,
}
impl HydratorState {
    pub fn consume(
        &mut self,
        action: BuildEventAction<bazel_event::BazelBuildEvent>,
    ) -> Vec<Option<HydratedInfo>> {
        match action {
            BuildEventAction::BuildCompleted => {
                self.rule_kind_lookup.clear();
                vec![None]
            }
            BuildEventAction::LifecycleEvent(_) => Vec::default(),
            BuildEventAction::BuildEvent(msg) => match msg.event {
                bazel_event::Evt::BazelEvent(_) => Vec::default(),
                bazel_event::Evt::TargetConfigured(tgt_cfg) => {
                    self.rule_kind_lookup
                        .insert(tgt_cfg.label, tgt_cfg.rule_kind);
                    Vec::default()
                }

                bazel_event::Evt::NamedSetOfFiles {
                    id,
                    named_set_of_files,
                } => {
                    let _ = {
                        self.named_set_of_files_lookup
                            .insert(id, named_set_of_files)
                    };

                    let tmp_v: Vec<bazel_event::TargetCompletedEvt> =
                        self.buffered_tce.drain(..).collect();

                    let mut r = vec![];
                    for tce in tmp_v.into_iter() {
                        if let Some(target_complete_info) = tce_event(
                            tce,
                            &self.rule_kind_lookup,
                            &self.named_set_of_files_lookup,
                            &mut self.buffered_tce,
                        ) {
                            r.push(Some(HydratedInfo::TargetComplete(target_complete_info)))
                        }
                    }
                    r
                }
                bazel_event::Evt::TargetCompleted(tce) => {
                    if let Some(target_complete_info) = tce_event(
                        tce,
                        &self.rule_kind_lookup,
                        &self.named_set_of_files_lookup,
                        &mut self.buffered_tce,
                    ) {
                        vec![Some(HydratedInfo::TargetComplete(target_complete_info))]
                    } else {
                        Vec::default()
                    }
                }

                bazel_event::Evt::ActionCompleted(ace) => {
                    if !ace.success {
                        let err_info = ActionFailedErrorInfo {
                            stdout: ace.stdout.map(|stdout| build_event_stream::File {
                                file: Some(stdout),
                                path_prefix: vec![],
                                name: String::from("stdout"),
                                digest: String::default(),
                                length: -1,
                            }),
                            stderr: ace.stderr.map(|stderr| build_event_stream::File {
                                file: Some(stderr),
                                path_prefix: vec![],
                                name: String::from("stderr"),
                                digest: String::default(),
                                length: -1,
                            }),
                            target_kind: self.rule_kind_lookup.get(&ace.label).cloned(),
                            label: ace.label,
                        };
                        vec![Some(HydratedInfo::ActionFailed(err_info))]
                    } else {
                        let act_info = ActionSuccessInfo {
                            stdout: ace.stdout.map(|stdout| build_event_stream::File {
                                file: Some(stdout),
                                path_prefix: vec![],
                                name: String::from("stdout"),
                                digest: String::default(),
                                length: -1,
                            }),
                            stderr: ace.stderr.map(|stderr| build_event_stream::File {
                                file: Some(stderr),
                                path_prefix: vec![],
                                name: String::from("stderr"),
                                digest: String::default(),
                                length: -1,
                            }),

                            target_kind: self.rule_kind_lookup.get(&ace.label).cloned(),
                            label: ace.label,
                        };

                        vec![Some(HydratedInfo::ActionSuccess(act_info))]
                    }
                }

                bazel_event::Evt::TestResult(tfe) => {
                    let tst_info = TestResultInfo {
                        target_kind: self.rule_kind_lookup.get(&tfe.label).cloned(),
                        test_summary_event: tfe,
                    };

                    vec![Some(HydratedInfo::TestResult(tst_info))]
                }
                bazel_event::Evt::Progress(progress) => {
                    vec![Some(HydratedInfo::Progress(progress))]
                }
                bazel_event::Evt::Aborted(tfe) => {
                    let err_info = BazelAbortErrorInfo {
                        reason: tfe.reason,
                        description: tfe.description,
                        label: tfe.label,
                    };
                    vec![Some(HydratedInfo::BazelAbort(err_info))]
                }
                bazel_event::Evt::UnknownEvent(_) => Vec::default(),
            },
        }
    }
}

impl HydratedInfo {
    pub fn build_transformer(
        rx: async_channel::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,
    ) -> async_channel::Receiver<Option<HydratedInfo>> {
        let (tx, next_rx) = async_channel::unbounded();
        let mut hydrator = HydratorState::default();
        tokio::spawn(async move {
            while let Ok(action) = rx.recv().await {
                for r in hydrator.consume(action) {
                    tx.send(r).await.unwrap();
                }
            }
        });
        next_rx
    }

    pub fn label(&self) -> Option<&str> {
        match self {
            HydratedInfo::BazelAbort(ba) =>
            // this is a dance to return the ref
            {
                match &ba.label {
                    Some(s) => Some(s.as_str()),
                    None => None,
                }
            }
            HydratedInfo::ActionFailed(af) => Some(af.label.as_str()),
            HydratedInfo::Progress(_) => None,
            HydratedInfo::TestResult(tri) => Some(tri.test_summary_event.label.as_str()),
            HydratedInfo::ActionSuccess(asucc) => Some(asucc.label.as_str()),
            HydratedInfo::TargetComplete(tc) => Some(tc.label.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_no_history() {
        let (tx, rx) = async_channel::unbounded();
        let mut child_rx = std::pin::pin!(HydratedInfo::build_transformer(rx));

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: None,
                stderr: None,
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .await
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                stderr: None,
                stdout: None
            }))
        );
    }

    #[tokio::test]
    async fn test_with_files() {
        let (tx, rx) = async_channel::unbounded();
        let mut child_rx = std::pin::pin!(HydratedInfo::build_transformer(rx));

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .await
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                stderr: Some(build_event_stream::File {
                    name: String::from("stderr"),
                    path_prefix: Vec::default(),
                    digest: String::default(),
                    length: -1,
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stderr"
                    )))
                }),

                stdout: Some(build_event_stream::File {
                    name: String::from("stdout"),
                    path_prefix: Vec::default(),
                    digest: String::default(),
                    length: -1,
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stdout"
                    )))
                }),
            }))
        );
    }

    #[tokio::test]
    async fn test_with_history() {
        let (tx, rx) = async_channel::unbounded();
        let mut child_rx = std::pin::pin!(HydratedInfo::build_transformer(rx));

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
        .await
        .unwrap();

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .await
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: Some(String::from("my_madeup_rule")),
                label: String::from("foo_bar_baz"),
                stderr: Some(build_event_stream::File {
                    name: String::from("stderr"),
                    digest: String::default(),
                    length: -1,
                    path_prefix: Vec::default(),
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stderr"
                    )))
                }),

                stdout: Some(build_event_stream::File {
                    name: String::from("stdout"),
                    digest: String::default(),
                    length: -1,
                    path_prefix: Vec::default(),
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stdout"
                    )))
                }),
            }))
        );
    }

    #[tokio::test]
    async fn state_resets_on_new_build() {
        let (tx, rx) = async_channel::unbounded();
        let mut child_rx = std::pin::pin!(HydratedInfo::build_transformer(rx));

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
        .await
        .unwrap();

        tx.send(BuildEventAction::BuildCompleted).await.unwrap();

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stdout",
                ))),
                stderr: Some(build_event_stream::file::File::Uri(String::from(
                    "path-to-stderr",
                ))),
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .await
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        // First event is a None to indicate the build is completed.
        assert_eq!(received_res, None);

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                stderr: Some(build_event_stream::File {
                    name: String::from("stderr"),
                    path_prefix: Vec::default(),
                    digest: String::default(),
                    length: -1,
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stderr"
                    )))
                }),

                stdout: Some(build_event_stream::File {
                    name: String::from("stdout"),
                    path_prefix: Vec::default(),
                    digest: String::default(),
                    length: -1,
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "path-to-stdout"
                    )))
                }),
            }))
        );
    }
}
