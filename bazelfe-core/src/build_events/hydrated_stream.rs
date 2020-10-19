// Not entirely sure one would want to keep these layers/separation long term
// right now this separation in writing this makes it easy to catalog the function
// and ensure its tested right.

// maps over the action stream and provides a new stream of just ErrorInfo outputs
// Unknown if we should consume this as a stream and try action failures immediately
// or wait till the operation is done not to mutate things under bazel?

use std::collections::HashMap;

use super::build_event_server::bazel_event;
use super::build_event_server::BuildEventAction;
use bazelfe_protos::*;

use tokio::sync::broadcast;
use tokio::sync::mpsc;

// This is keeping some state as we go through a stream to hydrate values with things like rule kinds
// not on the indvidual events.

#[derive(Clone, PartialEq, Debug)]
pub struct ActionFailedErrorInfo {
    pub label: String,
    pub output_files: Vec<build_event_stream::file::File>,
    pub target_kind: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct BazelAbortErrorInfo {
    pub label: Option<String>,
    pub reason: Option<build_event_stream::aborted::AbortReason>,
    pub description: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ActionSuccessInfo {
    pub label: String,
    pub stdout: Option<build_event_stream::file::File>,
    pub stderr: Option<build_event_stream::file::File>,
    pub target_kind: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TargetCompleteInfo {
    pub label: String,
    pub success: bool,
    pub target_kind: Option<String>,
    pub output_files: Vec<build_event_stream::file::File>,
}

// Broad strokes of the failure occured inside an action (most common)
// or at a bazel abort, things like mis-configured build files
#[derive(Clone, PartialEq, Debug)]
pub enum HydratedInfo {
    BazelAbort(BazelAbortErrorInfo),
    ActionFailed(ActionFailedErrorInfo),
    Progress(bazel_event::ProgressEvt),
    ActionSuccess(ActionSuccessInfo),
    TargetComplete(TargetCompleteInfo),
}

fn recursive_lookup(
    lut: &HashMap<String, build_event_stream::NamedSetOfFiles>,
    results: &mut Vec<build_event_stream::file::File>,
    mut ids: Vec<String>,
) -> bool {
    while !ids.is_empty() {
        if let Some(head) = ids.pop() {
            if let Some(r) = lut.get(&head) {
                results.extend(
                    r.files
                        .iter()
                        .flat_map(|e| e.file.as_ref().map(|e| e.clone())),
                );
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
    let found_everything = if let Some(output_grp) = &tce
        .output_groups
        .iter()
        .filter(|grp| grp.name == "default")
        .next()
    {
        recursive_lookup(
            &named_set_of_files_lookup,
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
            output_files: output_files,
            target_kind: rule_kind_lookup.get(&tce.label).map(|e| e.clone()),
            label: tce.label,
            success: tce.success,
        };
        Some(target_complete_info)
    } else {
        to_revisit.push(tce);
        None
    }
}

impl HydratedInfo {
    pub fn build_transformer(
        mut rx: broadcast::Receiver<BuildEventAction<bazel_event::BazelBuildEvent>>,
    ) -> mpsc::Receiver<Option<HydratedInfo>> {
        let (mut tx, next_rx) = mpsc::channel(256);

        tokio::spawn(async move {
            let mut rule_kind_lookup = HashMap::new();
            let mut named_set_of_files_lookup = HashMap::new();
            let mut buffered_tce: Vec<bazel_event::TargetCompletedEvt> = Vec::default();

            while let Ok(action) = rx.recv().await {
                match action {
                    BuildEventAction::BuildCompleted => {
                        rule_kind_lookup.clear();
                        tx.send(None).await.unwrap();
                    }
                    BuildEventAction::LifecycleEvent(_) => (),
                    BuildEventAction::BuildEvent(msg) => match msg.event {
                        bazel_event::Evt::BazelEvent(_) => (),
                        bazel_event::Evt::TargetConfigured(tgt_cfg) => {
                            rule_kind_lookup.insert(tgt_cfg.label, tgt_cfg.rule_kind);
                        }

                        bazel_event::Evt::NamedSetOfFiles {
                            id,
                            named_set_of_files,
                        } => {
                            named_set_of_files_lookup.insert(id, named_set_of_files);

                            let tmp_v: Vec<bazel_event::TargetCompletedEvt> =
                                buffered_tce.drain(..).collect();
                            for tce in tmp_v.into_iter() {
                                if let Some(target_complete_info) = tce_event(
                                    tce,
                                    &rule_kind_lookup,
                                    &named_set_of_files_lookup,
                                    &mut buffered_tce,
                                ) {
                                    tx.send(Some(HydratedInfo::TargetComplete(
                                        target_complete_info,
                                    )))
                                    .await
                                    .unwrap();
                                }
                            }
                        }
                        bazel_event::Evt::TargetCompleted(tce) => {
                            if let Some(target_complete_info) = tce_event(
                                tce,
                                &rule_kind_lookup,
                                &named_set_of_files_lookup,
                                &mut buffered_tce,
                            ) {
                                tx.send(Some(HydratedInfo::TargetComplete(target_complete_info)))
                                    .await
                                    .unwrap();
                            }
                        }

                        bazel_event::Evt::ActionCompleted(ace) => {
                            if !ace.success {
                                let err_info = ActionFailedErrorInfo {
                                    output_files: ace
                                        .stdout
                                        .into_iter()
                                        .chain(ace.stderr.into_iter())
                                        .collect(),
                                    target_kind: rule_kind_lookup
                                        .get(&ace.label)
                                        .map(|e| e.clone()),
                                    label: ace.label,
                                };
                                tx.send(Some(HydratedInfo::ActionFailed(err_info)))
                                    .await
                                    .unwrap();
                            } else {
                                let act_info = ActionSuccessInfo {
                                    stdout: ace.stdout,
                                    stderr: ace.stderr,

                                    target_kind: rule_kind_lookup
                                        .get(&ace.label)
                                        .map(|e| e.clone()),
                                    label: ace.label,
                                };
                                tx.send(Some(HydratedInfo::ActionSuccess(act_info)))
                                    .await
                                    .unwrap();
                            }
                        }

                        bazel_event::Evt::TestFailure(tfe) => {
                            let err_info = ActionFailedErrorInfo {
                                output_files: tfe.failed_files,
                                target_kind: rule_kind_lookup.get(&tfe.label).map(|e| e.clone()),
                                label: tfe.label,
                            };
                            tx.send(Some(HydratedInfo::ActionFailed(err_info)))
                                .await
                                .unwrap();
                        }
                        bazel_event::Evt::Progress(progress) => {
                            tx.send(Some(HydratedInfo::Progress(progress)))
                                .await
                                .unwrap();
                        }
                        bazel_event::Evt::Aborted(tfe) => {
                            let err_info = BazelAbortErrorInfo {
                                reason: tfe.reason,
                                description: tfe.description,
                                label: tfe.label,
                            };
                            tx.send(Some(HydratedInfo::BazelAbort(err_info)))
                                .await
                                .unwrap();
                        }
                        bazel_event::Evt::UnknownEvent(_) => (),
                    },
                }
            }
        });
        next_rx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    #[tokio::test]
    async fn test_no_history() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = HydratedInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::ActionCompleted(bazel_event::ActionCompletedEvt {
                stdout: None,
                stderr: None,
                label: String::from("foo_bar_baz"),
                success: false,
            }),
        }))
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                output_files: vec![]
            }))
        );
    }

    #[tokio::test]
    async fn test_with_files() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = HydratedInfo::build_transformer(rx);

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
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: None,
                label: String::from("foo_bar_baz"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }

    #[tokio::test]
    async fn test_with_history() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = HydratedInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
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
        .unwrap();

        let received_res = child_rx.next().await.unwrap();

        assert_eq!(
            received_res,
            Some(HydratedInfo::ActionFailed(ActionFailedErrorInfo {
                target_kind: Some(String::from("my_madeup_rule")),
                label: String::from("foo_bar_baz"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }

    #[tokio::test]
    async fn state_resets_on_new_build() {
        let (tx, rx) = broadcast::channel(128);
        let mut child_rx = HydratedInfo::build_transformer(rx);

        tx.send(BuildEventAction::BuildEvent(bazel_event::BazelBuildEvent {
            event: bazel_event::Evt::TargetConfigured(bazel_event::TargetConfiguredEvt {
                label: String::from("foo_bar_baz"),
                rule_kind: String::from("my_madeup_rule"),
            }),
        }))
        .unwrap();

        tx.send(BuildEventAction::BuildCompleted).unwrap();

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
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("path-to-stdout",)),
                    build_event_stream::file::File::Uri(String::from("path-to-stderr",))
                ]
            }))
        );
    }
}
