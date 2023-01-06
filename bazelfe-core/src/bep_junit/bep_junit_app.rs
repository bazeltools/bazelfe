use bazelfe_bazel_wrapper::bep::build_events::build_event_server::BuildEventAction;
use bazelfe_core::bep_junit::{
    emit_backup_error_data, emit_junit_xml_from_aborted_action, emit_junit_xml_from_failed_action,
    label_to_junit_relative_path, suites_with_error_from_xml,
};

use bazelfe_bazel_wrapper::bep::build_events::build_event_server::bazel_event::BazelBuildEvent;
use bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratorState;
use bazelfe_protos::build_event_stream::BuildEvent;
use clap::Parser;
use prost::Message;
use std::collections::VecDeque;
use std::error::Error;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[clap(name = "bep-junit")]
struct Opt {
    #[clap(long)]
    build_event_binary_output: PathBuf,

    #[clap(long)]
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

    let mut failed_actions = Vec::default();
    let mut aborted_actions = Vec::default();
    let mut failed_tests = Vec::default();
    for build_event in res.iter() {
        match build_event {
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::BazelAbort(abort_info) => {
                emit_junit_xml_from_aborted_action(
                    abort_info,
                    aborted_actions.len(),
                    &opt.junit_output_path,
                );
                aborted_actions.push(abort_info.label.clone());
            }
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::ActionFailed(
                action_failed,
            ) => {
                emit_junit_xml_from_failed_action(action_failed, &opt.junit_output_path);
                failed_actions.push(action_failed.label.clone());
            }
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::Progress(_) => (),
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::TestResult(r) => {
                if let bazelfe_bazel_wrapper::bep::build_events::build_event_server::bazel_event::TestStatus::Failed =  r.test_summary_event.test_status {
                    failed_tests.push(r.test_summary_event.label.clone());
                }
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
                                .unwrap_or_else(|| panic!("Wasn't a local file for {}", uri));
                            if p.ends_with("/test.xml") {
                                Some(p)
                            } else {
                                None
                            }
                        }
                        bazelfe_protos::build_event_stream::file::File::Contents(_) => None,
                    })
                    .collect();

                let have_errors = files.iter().any(|result_xml| {
                    suites_with_error_from_xml(
                        std::fs::File::open(result_xml)
                            .expect("Should be able to open the found xml"),
                    )
                });

                if have_errors {
                    emit_backup_error_data(r, &opt.junit_output_path);
                }
                for (idx, f) in files.iter().enumerate() {
                    let output_file = output_folder.join(format!("test.{}.xml", idx));
                    std::fs::copy(f, output_file).unwrap();
                }
            }
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::ActionSuccess(_) => (),
            bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::HydratedInfo::TargetComplete(_) => (),
        }
    }

    if failed_actions.is_empty() && failed_tests.is_empty() && aborted_actions.is_empty() {
        println!("Have zero failures, all successful.")
    } else {
        if !failed_actions.is_empty() {
            println!("Have {} failed actions", failed_actions.len());
            for a in failed_actions.iter() {
                println!("  - {}", a);
            }
        }

        if !failed_tests.is_empty() {
            println!("Have {} failed tests", failed_tests.len());
            for a in failed_tests.iter() {
                println!("  - {}", a);
            }
        }

        if !aborted_actions.is_empty() {
            println!("Have {} aborted actions", aborted_actions.len());
            for a in aborted_actions.iter() {
                println!(
                    "  - {}",
                    a.to_owned().unwrap_or_else(|| "Unknown".to_string())
                );
            }
        }
    }
    Ok(())
}
