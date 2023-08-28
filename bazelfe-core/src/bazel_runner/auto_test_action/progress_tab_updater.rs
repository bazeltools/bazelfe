use std::time::Instant;

use bazelfe_bazel_wrapper::bep::{build_events, BazelEventHandler};
use build_events::hydrated_stream::HasFiles;

use crate::hydrated_stream_processors::BuildEventResponse;

#[derive(Debug)]
pub struct ProgressTabUpdater {
    progress_pump: flume::Sender<String>,
    action_event_tx: flume::Sender<super::ActionTargetStateScrollEntry>,
}

impl ProgressTabUpdater {
    pub fn new(
        progress_pump: flume::Sender<String>,
        action_event_tx: flume::Sender<super::ActionTargetStateScrollEntry>,
    ) -> Self {
        Self {
            progress_pump,
            action_event_tx,
        }
    }
}

#[async_trait::async_trait]
impl BazelEventHandler<BuildEventResponse> for ProgressTabUpdater {
    async fn process_event(
        &self,
        bazel_run_id: usize,
        event: &build_events::hydrated_stream::HydratedInfo,
    ) -> Vec<crate::hydrated_stream_processors::BuildEventResponse> {
        match event {
            build_events::hydrated_stream::HydratedInfo::Progress(p) => {
                if !p.stderr.is_empty() {
                    let _ = self.progress_pump.send_async(p.stderr.clone()).await;
                }
                if !p.stdout.is_empty() {
                    let _ = self.progress_pump.send_async(p.stdout.clone()).await;
                }
            }

            build_events::hydrated_stream::HydratedInfo::BazelAbort(_ba) => {}
            build_events::hydrated_stream::HydratedInfo::ActionFailed(af) => {
                let _ = self
                    .action_event_tx
                    .send_async(super::ActionTargetStateScrollEntry {
                        complete_type: super::CompleteKind::Action,
                        success: false,
                        label: af.label.clone(),
                        when: Instant::now(),
                        files: af.files(),
                        target_kind: af.target_kind.clone(),
                        bazel_run_id,
                    })
                    .await;
            }
            build_events::hydrated_stream::HydratedInfo::ActionSuccess(action_success) => {
                let _ = self
                    .action_event_tx
                    .send_async(super::ActionTargetStateScrollEntry {
                        complete_type: super::CompleteKind::Action,
                        success: true,
                        label: action_success.label.clone(),
                        when: Instant::now(),
                        files: action_success.files(),
                        target_kind: action_success.target_kind.clone(),
                        bazel_run_id,
                    })
                    .await;
            }
            build_events::hydrated_stream::HydratedInfo::TargetComplete(tc) => {
                let _ = self
                    .action_event_tx
                    .send_async(super::ActionTargetStateScrollEntry {
                        complete_type: super::CompleteKind::Target,
                        success: tc.success,
                        label: tc.label.clone(),
                        when: Instant::now(),
                        target_kind: tc.target_kind.clone(),
                        files: tc.output_files.clone(),
                        bazel_run_id,
                    })
                    .await;
            }
            build_events::hydrated_stream::HydratedInfo::TestResult(tst) => {
                let is_success = match tst.test_summary_event.test_status {
                    build_events::build_event_server::bazel_event::TestStatus::Passed => true,
                    build_events::build_event_server::bazel_event::TestStatus::Flaky => false,
                    build_events::build_event_server::bazel_event::TestStatus::Timeout => false,
                    build_events::build_event_server::bazel_event::TestStatus::Failed => false,
                    build_events::build_event_server::bazel_event::TestStatus::Incomplete => false,
                    build_events::build_event_server::bazel_event::TestStatus::RemoteFailure => false,
                    build_events::build_event_server::bazel_event::TestStatus::FailedToBuild => false,
                    build_events::build_event_server::bazel_event::TestStatus::ToolHaltedBeforeTesting => false,
                };
                let output_files = tst
                    .test_summary_event
                    .output_files
                    .iter()
                    .filter(|e| match e {
                        bazelfe_protos::build_event_stream::file::File::Uri(u) => {
                            u.ends_with(".log")
                        }
                        bazelfe_protos::build_event_stream::file::File::Contents(_) => true,
                    })
                    .map(|f| bazelfe_protos::build_event_stream::File {
                        file: Some(f.clone()),
                        path_prefix: Vec::default(),
                        name: "stderr".to_string(),
                        digest: String::default(),
                        length: -1,
                    })
                    .collect();
                let _ = self
                    .action_event_tx
                    .send_async(super::ActionTargetStateScrollEntry {
                        complete_type: super::CompleteKind::Test,
                        success: is_success,
                        label: tst.test_summary_event.label.clone(),
                        when: Instant::now(),
                        files: output_files,
                        target_kind: tst.target_kind.clone(),
                        bazel_run_id,
                    })
                    .await;
            }
        }
        Vec::default()
    }
}
