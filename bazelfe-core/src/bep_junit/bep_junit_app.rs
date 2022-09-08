use bazelfe_core::bep_junit::{emit_junit_xml_from_failed_action, label_to_junit_relative_path};
use bazelfe_core::build_events::build_event_server::BuildEventAction;

use bazelfe_core::build_events::build_event_server::bazel_event::BazelBuildEvent;
use bazelfe_core::build_events::hydrated_stream::HydratorState;
use bazelfe_protos::build_event_stream::BuildEvent;
use clap::Parser;
use prost::Message;
use std::collections::VecDeque;
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[clap(name = "bep-junit")]
struct Opt {
    #[clap(long, parse(from_os_str))]
    build_event_binary_output: PathBuf,

    #[clap(long, parse(from_os_str))]
    junit_output_path: PathBuf,
}

fn load_build_event_proto(d: &Path) -> impl Iterator<Item = BuildEvent> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let r = load_build_event_proto(opt.build_event_binary_output.as_path());

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
                emit_junit_xml_from_failed_action(action_failed, &opt.junit_output_path);
            }
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::Progress(_) => (),
            bazelfe_core::build_events::hydrated_stream::HydratedInfo::TestResult(r) => {
                let output_folder = opt.junit_output_path.join(label_to_junit_relative_path(
                    r.test_summary_event.label.as_str(),
                ));
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