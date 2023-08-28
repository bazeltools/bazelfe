use std::path::PathBuf;

use bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::{
    ActionFailedErrorInfo, ActionSuccessInfo, HasFiles,
};

pub(in crate::hydrated_stream_processors::process_bazel_failures) async fn text_logs_from_success(
    action_success_info: &ActionSuccessInfo,
) -> Vec<String> {
    let mut error_data = Vec::default();
    for path_str in action_success_info.path_bufs().into_iter() {
        let path: PathBuf = path_str;
        if path.exists() {
            let file_len = std::fs::metadata(&path).unwrap().len();
            if file_len < 10 * 1024 * 1024 {
                // 10 MB
                error_data.push(tokio::fs::read_to_string(&path).await.unwrap());
            }
        }
    }
    error_data
}

pub(in crate::hydrated_stream_processors::process_bazel_failures) async fn text_logs_from_failure(
    action_failed_error_info: &ActionFailedErrorInfo,
) -> Vec<String> {
    let mut error_data = Vec::default();
    for path_str in action_failed_error_info.path_bufs().into_iter() {
        let path: PathBuf = path_str;
        if path.exists() {
            let file_len = std::fs::metadata(&path).unwrap().len();
            if file_len < 10 * 1024 * 1024 {
                // 10 MB
                error_data.push(tokio::fs::read_to_string(&path).await.unwrap());
            }
        }
    }
    error_data
}
