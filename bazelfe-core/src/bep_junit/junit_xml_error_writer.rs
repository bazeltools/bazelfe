use xml::writer::XmlEvent;

pub trait XmlWritable {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>);
}

#[derive(Debug, PartialEq)]
pub struct TestSuites {
    pub testsuites: Vec<TestSuite>,
}
impl XmlWritable for TestSuites {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
        let e = XmlEvent::start_element("testsuites");
        writer.write(e).unwrap();

        for s in self.testsuites.iter() {
            s.write_xml(writer);
        }
        writer.write(XmlEvent::end_element()).unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub struct TestSuite {
    pub name: String,
    pub tests: u32,
    pub failures: u32,
    pub testcases: Vec<TestCase>,
}
impl XmlWritable for TestSuite {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
        let tests = self.tests.to_string();
        let failures = self.failures.to_string();
        let e = XmlEvent::start_element("testsuite")
            .attr("name", self.name.as_str())
            .attr("tests", tests.as_str())
            .attr("failures", failures.as_str());

        writer.write(e).unwrap();

        for s in self.testcases.iter() {
            s.write_xml(writer);
        }
        writer.write(XmlEvent::end_element()).unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub struct TestCase {
    pub name: String,
    pub time: f64,
    pub failures: Vec<Failure>,
}

impl XmlWritable for TestCase {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
        let time = self.time.to_string();
        let e = XmlEvent::start_element("testcase")
            .attr("name", self.name.as_str())
            .attr("time", time.as_str());

        writer.write(e).unwrap();

        for s in self.failures.iter() {
            s.write_xml(writer);
        }
        writer.write(XmlEvent::end_element()).unwrap();
    }
}

#[derive(Debug, PartialEq)]
pub struct Failure {
    pub message: String,
    pub tpe_name: String,
    pub value: String,
}

impl XmlWritable for Failure {
    fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
        let e = XmlEvent::start_element("failure")
            .attr("message", self.message.as_str())
            .attr("type", self.tpe_name.as_str());

        writer.write(e).unwrap();

        writer.write(XmlEvent::CData(self.value.as_str())).unwrap();
        writer.write(XmlEvent::end_element()).unwrap();
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn xml_writable_to_string<T: XmlWritable>(t: &T) -> String {
        let mut v = Vec::default();
        let mut xml_writer = xml::writer::EventWriter::new(&mut v);
        t.write_xml(&mut xml_writer);
        drop(xml_writer);
        String::from_utf8(v).expect("Should emit sane UTF-8")
    }

    #[test]
    fn test_failure_serialization() {
        let f = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build".to_string(),
        };

        assert_eq!(
            xml_writable_to_string(&f),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><failure message=\"Failed to build\" type=\"BuildFailure\"><![CDATA[System failed to build]]></failure>".to_string()
        );
    }

    #[test]
    fn test_testsuites_serialization() {
        let f = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build".to_string(),
        };

        let v = TestSuites {
            testsuites: vec![TestSuite {
                name: "SuiteA".to_string(),
                tests: 3,
                failures: 1,
                testcases: vec![TestCase {
                    name: "TestCaseA".to_string(),
                    time: 0.3,
                    failures: vec![f],
                }],
            }],
        };
        assert_eq!(
            xml_writable_to_string(&v),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><testsuites><testsuite name=\"SuiteA\" tests=\"3\" failures=\"1\"><testcase name=\"TestCaseA\" time=\"0.3\"><failure message=\"Failed to build\" type=\"BuildFailure\"><![CDATA[System failed to build]]></failure></testcase></testsuite></testsuites>".to_string()
        );
    }
}
