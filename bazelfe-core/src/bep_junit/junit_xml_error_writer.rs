use xml::writer::XmlEvent;

    #[derive(Debug)]
    pub struct TestSuites {
        pub testsuites: Vec<TestSuite>,
    }
    impl TestSuites {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
            let e = XmlEvent::start_element("testsuites");
            writer.write(e).unwrap();

            for s in self.testsuites.iter() {
                s.write_xml(writer);
            }
            writer.write(XmlEvent::end_element()).unwrap();
        }
    }

    #[derive(Debug)]
    pub struct TestSuite {
        pub name: String,
        pub tests: u32,
        pub failures: u32,
        pub testcases: Vec<TestCase>,
    }
    impl TestSuite {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
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

    #[derive(Debug)]
    pub struct TestCase {
        pub name: String,
        pub time: f64,
        pub failures: Vec<Failure>,
    }

    impl TestCase {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
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

    #[derive(Debug)]
    pub struct Failure {
        pub message: String,
        pub tpe_name: String,
        pub value: String,
    }

    impl Failure {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
            let e = XmlEvent::start_element("failure")
                .attr("message", self.message.as_str())
                .attr("type", self.tpe_name.as_str());

            writer.write(e).unwrap();

            writer.write(XmlEvent::CData(self.value.as_str())).unwrap();
            writer.write(XmlEvent::end_element()).unwrap();
        }
    }