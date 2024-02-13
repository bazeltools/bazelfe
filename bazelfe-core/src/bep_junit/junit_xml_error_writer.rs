use xml::writer::{Error as XmlError, XmlEvent};

use super::xml_utils::XmlWritable;

#[derive(Debug, PartialEq)]
pub struct TestSuites {
    pub testsuites: Vec<TestSuite>,
}
impl XmlWritable for TestSuites {
    fn write_xml<W: std::io::Write>(
        &self,
        writer: &mut xml::writer::EventWriter<W>,
    ) -> Result<(), XmlError> {
        let e = XmlEvent::start_element("testsuites");
        writer.write(e)?;

        for s in self.testsuites.iter() {
            s.write_xml(writer)?;
        }
        writer.write(XmlEvent::end_element())
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
    fn write_xml<W: std::io::Write>(
        &self,
        writer: &mut xml::writer::EventWriter<W>,
    ) -> Result<(), XmlError> {
        let tests = self.tests.to_string();
        let failures = self.failures.to_string();
        let e = XmlEvent::start_element("testsuite")
            .attr("name", self.name.as_str())
            .attr("tests", tests.as_str())
            .attr("failures", failures.as_str());

        writer.write(e)?;

        for s in self.testcases.iter() {
            s.write_xml(writer)?;
        }
        writer.write(XmlEvent::end_element())
    }
}

#[derive(Debug, PartialEq)]
pub struct TestCase {
    pub name: String,
    pub time: f64,
    pub failures: Vec<Failure>,
}

impl XmlWritable for TestCase {
    fn write_xml<W: std::io::Write>(
        &self,
        writer: &mut xml::writer::EventWriter<W>,
    ) -> Result<(), XmlError> {
        let time = self.time.to_string();
        let e = XmlEvent::start_element("testcase")
            .attr("name", self.name.as_str())
            .attr("time", time.as_str());

        writer.write(e)?;

        for s in self.failures.iter() {
            s.write_xml(writer)?;
        }
        writer.write(XmlEvent::end_element())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Failure {
    pub message: String,
    pub tpe_name: String,
    pub value: String,
}

impl XmlWritable for Failure {
    fn write_xml<W: std::io::Write>(
        &self,
        writer: &mut xml::writer::EventWriter<W>,
    ) -> Result<(), XmlError> {
        let e = XmlEvent::start_element("failure")
            .attr("message", self.message.as_str())
            .attr("type", self.tpe_name.as_str());

        writer.write(e)?;

        let msg = self.value.as_str();
        if contains_disallowed_xml_chars(msg) || msg.contains("]]>") {
            // CDATA can't have escapes inside so here we use normal character data and
            // let the library escape
            writer.write(XmlEvent::Characters(msg))?;
        } else {
            // we should just be able to use a raw CData here without bothering to escape
            // which is easier to inspect
            writer.write(XmlEvent::CData(msg))?;
        }
        writer.write(XmlEvent::end_element())
    }
}

fn contains_disallowed_xml_chars(input: &str) -> bool {
    input.chars().any(|c| {
        let u = c as u32;
        // Convert character to its Unicode code point
        // Check for disallowed characters:
        // - Control characters except tab (U+0009), line feed (U+000A), and carriage return (U+000D)
        // - Null character (U+0000)
        // - Characters in the range U+007F to U+009F
        (u <= 0x001F && u != 0x0009 && u != 0x000A && u != 0x000D) || (0x007F <= u && u <= 0x009F)
    })
}

#[cfg(test)]
mod tests {

    use crate::bep_junit::xml_utils::xml_writable_to_string;

    use super::*;

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
    fn test_failure_with_control_serialization() {
        let f = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build\u{0000}".to_string(),
        };

        assert_eq!(
            xml_writable_to_string(&f),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><failure message=\"Failed to build\" type=\"BuildFailure\">System failed to build\0</failure>".to_string()
        );

        let f1 = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build]]>".to_string(),
        };

        assert_eq!(
            xml_writable_to_string(&f1),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><failure message=\"Failed to build\" type=\"BuildFailure\">System failed to build]]></failure>".to_string()
        );

        let f2 = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build <sometag>".to_string(),
        };

        assert_eq!(
            xml_writable_to_string(&f2),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><failure message=\"Failed to build\" type=\"BuildFailure\"><![CDATA[System failed to build <sometag>]]></failure>".to_string()
        );

        let f3 = Failure {
            message: "Failed to build".to_string(),
            tpe_name: "BuildFailure".to_string(),
            value: "System failed to build <sometag> and ]]>".to_string(),
        };

        assert_eq!(
            xml_writable_to_string(&f3),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?><failure message=\"Failed to build\" type=\"BuildFailure\">System failed to build &lt;sometag> and ]]></failure>".to_string()
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
