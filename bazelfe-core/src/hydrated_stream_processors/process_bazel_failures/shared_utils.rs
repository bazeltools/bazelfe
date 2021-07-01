use std::path::PathBuf;

use bazelfe_protos::build_event_stream;

use crate::build_events::hydrated_stream::{ActionFailedErrorInfo, ActionSuccessInfo};

pub(in crate::hydrated_stream_processors::process_bazel_failures) fn output_from_paths<'a>(
    files: impl Iterator<Item = &'a build_event_stream::file::File>,
) -> Vec<std::path::PathBuf> {
    files
        .flat_map(|e| match e {
            build_event_stream::file::File::Uri(e) => {
                if e.starts_with("file://") {
                    let u: PathBuf = e.strip_prefix("file://").unwrap().into();
                    Some(u)
                } else {
                    warn!("Path isn't a file, so skipping...{:?}", e);

                    None
                }
            }
            build_event_stream::file::File::Contents(_) => None,
        })
        .collect()
}

pub(in crate::hydrated_stream_processors::process_bazel_failures) fn output_success_paths(
    data: &ActionSuccessInfo,
) -> Vec<std::path::PathBuf> {
    output_from_paths(
        vec![&data.stdout, &data.stderr]
            .iter()
            .filter_map(|&e| e.as_ref())
            .filter_map(|e| e.file.as_ref()),
    )
}

pub(in crate::hydrated_stream_processors::process_bazel_failures) fn output_error_paths(
    err_data: &ActionFailedErrorInfo,
) -> Vec<std::path::PathBuf> {
    output_from_paths(err_data.files().iter().filter_map(|e| e.file.as_ref()))
}

pub(in crate::hydrated_stream_processors::process_bazel_failures) async fn text_logs_from_success(
    action_success_info: &ActionSuccessInfo,
) -> Vec<String> {
    let mut error_data = Vec::default();
    for path_str in output_success_paths(&action_success_info).into_iter() {
        let path: PathBuf = path_str.into();
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
    for path_str in output_error_paths(&action_failed_error_info).into_iter() {
        let path: PathBuf = path_str.into();
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
