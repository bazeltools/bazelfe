use std::path::Path;

use nom::combinator::fail;
use xml::reader::EventReader;
use xml::reader::XmlEvent;

use bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::TestResultInfo;

use super::junit_xml_error_writer;
use super::xml_utils::emit_junit_xml_from_failed_operation;

fn extract_file_content(test_result: &TestResultInfo) -> Vec<String> {
    let mut r = Vec::default();

    for inner_f in test_result.test_summary_event.output_files.iter() {
        match inner_f {
            bazelfe_protos::build_event_stream::file::File::Uri(uri) => {
                if let Some(p) = uri.strip_prefix("file://") {
                    if p.ends_with("/test.log") {
                        let s = std::fs::read_to_string(p)
                            .expect("Expected to be able to open input test data");
                        r.push(s);
                    }
                }
            }
            bazelfe_protos::build_event_stream::file::File::Contents(content) => {
                r.push(String::from_utf8_lossy(&content[..]).to_string());
            }
        }
    }
    r
}

pub fn emit_backup_error_data(test_result: &TestResultInfo, output_root: &Path) {
    if test_result.test_summary_event.test_status.didnt_pass() {
        let label_name = test_result.test_summary_event.label.clone();

        let desc = test_result.test_summary_event.test_status.description();
        // we have ran into issues with non-utf-8 characters in the output logs
        // so we will replace anything not-ascii to '?' cut it right back
        let output_data = extract_file_content(test_result).join("\n").replace(
            |c: char| !c.is_ascii() || (c != '\n' && c != '\r' && c.is_ascii_control()),
            "?",
        );

        let known_failures = vec![junit_xml_error_writer::Failure {
            message: format!("{} result: {}", label_name, desc),
            tpe_name: "ERROR".to_string(),
            value: output_data,
        }];

        let test_cases = vec![junit_xml_error_writer::TestCase {
            name: desc,
            time: 1.0f64,
            failures: known_failures,
        }];

        emit_junit_xml_from_failed_operation(test_cases, label_name, output_root)
    }
}

// In bazel if a test system.exits or otherwise exits abnormally we end up with xml files which are kinda useless with no output
// jenkins doesn't render these at all. So we want to detect for errors and then do something else externally to report these correctly
pub fn suites_with_error_from_xml<R: std::io::Read>(r: R) -> bool {
    let parser = EventReader::new(r);

    for e in parser {
        match e {
            Ok(XmlEvent::StartElement {
                name, attributes, ..
            }) => {
                let has_errors = attributes
                    .iter()
                    .find(|e| e.name.local_name == "errors" && e.value != "0");

                if name.local_name == "testsuite" && has_errors.is_some() {
                    return true;
                }
            }
            Err(e) => {
                panic!("Error: {}", e);
            }
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {

    use super::*;

    const SAMPLE_OUTPUT: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<testsuites>
  <testsuite
  errors="1" failures="0" hostname="samplehost" name="org.scalatest.DeferredAbortedSuite" tests="0" time="0.003" timestamp="2022-09-21T01:58:18">
    <properties>
      <property name="java.runtime.name" value="OpenJDK Runtime Environment">
      </property>
      <property name="java.vm.specification.version" value="1.8"> </property>
      <property name="sun.arch.data.model" value="64"> </property>

      <property name="java.specification.vendor" value="Oracle Corporation">
      </property>
      <property name="user.language" value="en"> </property>
      <property name="awt.toolkit" value="sun.awt.X11.XToolkit"> </property>
      <property name="java.vm.info" value="mixed mode"> </property>
      <property name="java.version" value="1.8.0_322"> </property>
      <property name="java.vendor" value="Azul Systems, Inc."> </property>
      <property name="file.separator" value="/"> </property>
      <property name="java.vendor.url.bug" value="http://www.azul.com/support/">
      </property>
      <property name="sun.cpu.endian" value="little"> </property>
      <property name="sun.io.unicode.encoding" value="UnicodeLittle"> </property>
      <property name="sun.cpu.isalist" value=""> </property>
    </properties>
    <system-out><![CDATA[]]></system-out>
    <system-err><![CDATA[]]></system-err>
</testsuite>
</testsuites>"#;

    const NEG_SAMPLE_OUTPUT: &str = r#"<?xml version="1.0" encoding="UTF-8" ?>
<testsuites>
  <testsuite
  errors="0" failures="1" hostname="samplehost" name="org.scalatest.DeferredAbortedSuite" tests="0" time="0.003" timestamp="2022-09-21T01:58:18">
    <system-out><![CDATA[]]></system-out>
    <system-err><![CDATA[]]></system-err>
</testsuite>
</testsuites>"#;

    #[test]
    fn test_extract_errors() {
        assert!(suites_with_error_from_xml(SAMPLE_OUTPUT.as_bytes()));
        assert!(!suites_with_error_from_xml(NEG_SAMPLE_OUTPUT.as_bytes()));
    }
}
