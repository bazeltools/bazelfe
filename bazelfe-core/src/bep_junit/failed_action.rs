use super::junit_xml_error_writer;
use super::label_to_junit_relative_path;
use super::xml_utils::emit_junit_xml_from_failed_operation;
use crate::bep_junit::xml_utils::XmlWritable;
use bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::{
    ActionFailedErrorInfo, BazelAbortErrorInfo,
};
use std::path::Path;

pub fn emit_junit_xml_from_aborted_action(
    aborted_evt: &BazelAbortErrorInfo,
    abort_idx: usize,
    output_root: &Path,
) {
    let label_name = aborted_evt
        .label
        .to_owned()
        .unwrap_or_else(|| format!("unknown-{}", abort_idx));

    let known_failures = vec![junit_xml_error_writer::Failure {
        message: format!("Failed to build, {}", label_name),
        tpe_name: "ERROR".to_string(),
        value: aborted_evt.description.clone(),
    }];

    let test_cases = vec![junit_xml_error_writer::TestCase {
        name: "Build aborted".to_string(),
        time: 1.0f64,
        failures: known_failures,
    }];

    emit_junit_xml_from_failed_operation(test_cases, label_name, output_root)
}

pub fn emit_junit_xml_from_failed_action(action: &ActionFailedErrorInfo, output_root: &Path) {
    emit_junit_xml_from_failed_operation(
        vec![generate_struct_from_failed_action(action)],
        action.label.clone(),
        output_root,
    )
}

fn generate_struct_from_failed_action(
    action: &ActionFailedErrorInfo,
) -> junit_xml_error_writer::TestCase {
    fn get_failure_type(
        known_failures: &mut Vec<junit_xml_error_writer::Failure>,
        nme: &str,
        f: &Option<bazelfe_protos::build_event_stream::File>,
    ) {
        if let Some(inner_f) = &f.as_ref().and_then(|e| e.file.as_ref()) {
            let mut str_v = None;

            match inner_f {
                bazelfe_protos::build_event_stream::file::File::Uri(uri) => {
                    if let Some(p) = uri.strip_prefix("file://") {
                        let s = std::fs::read_to_string(p)
                            .expect("Expected to be able to open input test data");
                        str_v = Some(s);
                    }
                }
                bazelfe_protos::build_event_stream::file::File::Contents(content) => {
                    str_v = Some(String::from_utf8_lossy(content).to_string())
                }
            }
            if let Some(content) = str_v {
                if !content.is_empty() {
                    known_failures.push(junit_xml_error_writer::Failure {
                        message: format!("Failed to build, {}", nme),
                        tpe_name: "ERROR".to_string(),
                        value: content,
                    });
                }
            }
        }
    }

    let mut known_failures = vec![];

    get_failure_type(&mut known_failures, "stderr", &action.stderr);
    get_failure_type(&mut known_failures, "stdout", &action.stdout);

    junit_xml_error_writer::TestCase {
        name: "Build failure".to_string(),
        time: 1.0f64,
        failures: known_failures,
    }
}

#[cfg(test)]
mod tests {

    use bazelfe_protos::build_event_stream;
    use junit_xml_error_writer::*;

    use super::*;
    #[test]
    fn test_generate_struct_from_failed_action() {
        let t = tempfile::tempdir().expect("Make a temp directory");
        use std::io::Write;

        let stdout_f = t.path().join("stdout_out");
        let mut file = std::fs::File::create(&stdout_f).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        let stderr_f = t.path().join("stderr_out");
        let mut file = std::fs::File::create(&stderr_f).unwrap();
        file.write_all(b"Hello, world!").unwrap();

        let v = ActionFailedErrorInfo {
            label: "//src/main/foo/bar:baz".to_string(),
            stdout: Some(build_event_stream::File {
                path_prefix: vec!["bazel-out".to_string()],
                name: "stdout".to_string(),
                digest: "AABE".to_string(),
                length: 33,
                file: Some(bazelfe_protos::build_event_stream::file::File::Uri(
                    format!("file://{}", stdout_f.to_string_lossy()),
                )),
            }),
            stderr: Some(build_event_stream::File {
                path_prefix: vec!["bazel-out".to_string()],
                name: "stderr".to_string(),
                digest: "AABEE".to_string(),
                length: 23,
                file: Some(bazelfe_protos::build_event_stream::file::File::Uri(
                    format!("file://{}", stderr_f.to_string_lossy()),
                )),
            }),
            target_kind: Some("my_test_type".to_string()),
        };
        assert_eq!(
            generate_struct_from_failed_action(&v),
            TestCase {
                name: "Build failure".to_string(),
                time: 1.0,
                failures: vec![
                    Failure {
                        message: "Failed to build, stderr".to_string(),
                        tpe_name: "ERROR".to_string(),
                        value: "Hello, world!".to_string()
                    },
                    Failure {
                        message: "Failed to build, stdout".to_string(),
                        tpe_name: "ERROR".to_string(),
                        value: "Hello, world!".to_string()
                    }
                ]
            }
        );
    }
}
