use bazelfe_core::build_events::build_event_server::BuildEventAction;

use bazelfe_core::build_events::build_event_server::bazel_event::BazelBuildEvent;
use bazelfe_core::build_events::hydrated_stream::{ActionFailedErrorInfo, HydratorState};
use bazelfe_protos::build_event_stream::BuildEvent;
use clap::Parser;
use prost::Message;
use rand::random;
use std::collections::VecDeque;
use std::error::Error;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[clap(name = "bep-junit")]
struct Opt {
    #[clap(long, parse(from_os_str))]
    build_event_binary_output: PathBuf,

    #[clap(long, parse(from_os_str))]
    junit_output_path: PathBuf,
}

fn load_proto(d: &Path) -> impl Iterator<Item = BuildEvent> {
    struct IterC(VecDeque<u8>);
    impl Iterator for IterC {
        type Item = BuildEvent;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                None
            } else {
                let decoded = BuildEvent::decode_length_delimited(&mut self.0).unwrap();
                Some(decoded)
            }
        }
    }

    let mut file = std::fs::File::open(d).expect("Expected to be able to open input test data");

    let mut data_vec = VecDeque::default();
    std::io::copy(&mut file, &mut data_vec).unwrap();

    IterC(data_vec)
}

fn label_to_child_path(label: &str) -> String {
    let p = if let Some(external_suffix) = label.strip_prefix("@") {
        format!("external/{}", external_suffix)
    } else if let Some(internal_suffix) = label.strip_prefix("//") {
        internal_suffix.to_string()
    } else {
        label.to_string()
    };

    p.replace("//", "/").replace(":", "/")
}

mod junit_testsuite {
    use xml::writer::XmlEvent;

    #[derive(Debug)]
    pub struct TestSuites {
        pub id: String,
        pub name: String,
        pub tests: u32,
        pub failures: u32,
        pub time: f64,
        pub testsuite: Vec<TestSuite>,
    }
    impl TestSuites {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
            let tests = self.tests.to_string();
            let failures = self.failures.to_string();
            let time = self.time.to_string();
            let e = XmlEvent::start_element("TestSuites")
                .attr("id", self.id.as_str())
                .attr("name", self.name.as_str())
                .attr("tests", tests.as_str())
                .attr("failures", failures.as_str())
                .attr("time", time.as_str());

            writer.write(e).unwrap();

            for s in self.testsuite.iter() {
                s.write_xml(writer);
            }
            writer.write(XmlEvent::end_element()).unwrap();
        }
    }

    #[derive(Debug)]
    pub struct TestSuite {
        pub id: String,
        pub name: String,
        pub tests: u32,
        pub failures: u32,
        pub time: f64,
        pub failure: Vec<Failure>,
    }

    impl TestSuite {
        pub fn write_xml<W: std::io::Write>(&self, writer: &mut xml::writer::EventWriter<W>) {
            let tests = self.tests.to_string();
            let failures = self.failures.to_string();
            let time = self.time.to_string();
            let e = XmlEvent::start_element("TestSuite")
                .attr("id", self.id.as_str())
                .attr("name", self.name.as_str())
                .attr("tests", tests.as_str())
                .attr("failures", failures.as_str())
                .attr("time", time.as_str());

            writer.write(e).unwrap();

            for s in self.failure.iter() {
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
            let e = XmlEvent::start_element("Failure")
                .attr("message", self.message.as_str())
                .attr("type", self.tpe_name.as_str());

            writer.write(e).unwrap();

            writer.write(XmlEvent::CData(self.value.as_str())).unwrap();
            writer.write(XmlEvent::end_element()).unwrap();
        }
    }
}

fn action_to_build_failure(action: &ActionFailedErrorInfo) -> junit_testsuite::TestSuite {
    fn get_failure_type(
        known_failures: &mut Vec<junit_testsuite::Failure>,
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
                    known_failures.push(junit_testsuite::Failure {
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

    junit_testsuite::TestSuite {
        id: random::<i32>().to_string(),
        name: action.label.clone(),
        tests: 1,
        failures: 1,
        time: 1.0f64,
        failure: known_failures,
    }
}

fn write_failed_action(action: &ActionFailedErrorInfo, output_root: &Path) {
    let output_folder = output_root.join(label_to_child_path(action.label.as_str()));
    let output_file = output_folder.join("test.xml");
    std::fs::create_dir_all(output_folder).expect("Make dir failed");
    let mut file = std::fs::File::create(&output_file)
        .expect(format!("Should open file {:?}", output_file).as_str());
    let e = junit_testsuite::TestSuites {
        id: random::<u32>().to_string(),
        name: action.label.clone(),
        tests: 1,
        failures: 1,
        time: 1.0f64,
        testsuite: vec![action_to_build_failure(action)],
    };
    use xml::writer::EventWriter;
    let mut event_writer = EventWriter::new(&mut file);
    e.write_xml(&mut event_writer);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let r = load_proto(opt.build_event_binary_output.as_path());

    std::fs::create_dir_all(&opt.junit_output_path).expect("Make output tree");

    let mut hydrator = HydratorState::default();
    let mut res = Vec::default();
    for e in r {
        let dec: BazelBuildEvent = e.into();
        res.extend(
            &mut hydrator
                .consume(BuildEventAction::BuildEvent(dec))
                .into_iter()
                .flatten(),
        );
    }
    res.extend(
        &mut hydrator
            .consume(BuildEventAction::BuildCompleted)
            .into_iter()
            .flatten(),
    );

    for build_event in res.iter() {
        match build_event {
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::BazelAbort(_) => (),
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::ActionFailed(
                action_failed,
            ) => {
                write_failed_action(action_failed, &opt.junit_output_path);
            }
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::Progress(_) => (),
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::TestResult(r) => {
                let output_folder = opt
                    .junit_output_path
                    .join(label_to_child_path(r.test_summary_event.label.as_str()));
                std::fs::create_dir_all(&output_folder).expect("Make dir failed");

                let files: Vec<&str> = r
                    .test_summary_event
                    .output_files
                    .iter()
                    .flat_map(|e| match e {
                        bazelfe_protos::build_event_stream::file::File::Uri(uri) => {
                            let p = uri
                                .strip_prefix("file://")
                                .expect(format!("Wasn't a local file for {}", uri).as_str());
                            if p.ends_with("/test.xml") {
                                Some(p)
                            } else {
                                None
                            }
                        }
                        bazelfe_protos::build_event_stream::file::File::Contents(_) => None,
                    })
                    .collect();
                for (idx, f) in files.iter().enumerate() {
                    let output_file = output_folder.join(format!("test.{}.xml", idx));
                    std::fs::copy(f, output_file).unwrap();
                }
            }
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::TargetComplete(_) => (),
        }
    }
    Ok(())
}
