use std::path::Path;

use crate::bep_junit::label_to_junit_relative_path;

use super::junit_xml_error_writer;

pub trait XmlWritable {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>);
}

pub fn xml_writable_to_string<T: XmlWritable>(t: &T) -> String {
    let mut v = Vec::default();
    let mut xml_writer = xml::writer::EventWriter::new(&mut v);
    t.write_xml(&mut xml_writer);
    drop(xml_writer);
    String::from_utf8(v).expect("Should emit sane UTF-8")
}

pub fn emit_junit_xml_from_failed_operation(
    test_cases: Vec<junit_xml_error_writer::TestCase>,
    label_name: String,
    output_root: &Path,
) {
    let output_folder = output_root.join(label_to_junit_relative_path(label_name.as_str()));
    let output_file = output_folder.join("test.xml");
    std::fs::create_dir_all(output_folder).expect("Make dir failed");
    let mut file = std::fs::File::create(&output_file)
        .unwrap_or_else(|_| panic!("Should open file {:?}", output_file));
    let e = junit_xml_error_writer::TestSuites {
        testsuites: vec![junit_xml_error_writer::TestSuite {
            name: label_name,
            tests: 1,
            failures: 1,
            testcases: test_cases,
        }],
    };
    use xml::writer::EventWriter;
    let mut event_writer = EventWriter::new(&mut file);
    e.write_xml(&mut event_writer);
}
