pub mod junit_xml_error_writer;

use crate::build_events::hydrated_stream::{ActionFailedErrorInfo};
use std::path::Path;

pub fn emit_junit_xml_from_failed_action(action: &ActionFailedErrorInfo, output_root: &Path) {
    let output_folder = output_root.join(label_to_junit_relative_path(action.label.as_str()));
    let output_file = output_folder.join("test.xml");
    std::fs::create_dir_all(output_folder).expect("Make dir failed");
    let mut file = std::fs::File::create(&output_file)
        .unwrap_or_else(|_| panic!("Should open file {:?}", output_file));
    let e = junit_xml_error_writer::TestSuites {
        testsuites: vec![junit_xml_error_writer::TestSuite {
            name: action.label.clone(),
            tests: 1,
            failures: 1,
            testcases: vec![generate_struct_from_failed_action(action)],
        }],
    };
    use xml::writer::EventWriter;
    let mut event_writer = EventWriter::new(&mut file);
    e.write_xml(&mut event_writer);
}

// This is to just take the label and provide a sane output path
// in the resulting junit root to avoid conflicts.
pub fn label_to_junit_relative_path(label: &str) -> String {
    let p: String = if let Some(external_suffix) = label.strip_prefix('@') {
        format!("external/{}", external_suffix)
    } else if let Some(internal_suffix) = label.strip_prefix("//") {
        internal_suffix.to_string()
    } else {
        label.to_string()
    };

    p.replace("//", "/").replace(":", "/")
}

fn generate_struct_from_failed_action(action: &ActionFailedErrorInfo) -> junit_xml_error_writer::TestCase {
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
        name: action.label.clone(),
        time: 1.0f64,
        failures: known_failures,
    }
}