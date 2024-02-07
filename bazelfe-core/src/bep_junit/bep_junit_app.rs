use bazelfe_bazel_wrapper::bep::build_events::build_event_server::BuildEventAction;
use bazelfe_core::bep_junit::{
    emit_backup_error_data, emit_junit_xml_from_aborted_action, emit_junit_xml_from_failed_action,
    label_to_junit_relative_path,
};

use bazelfe_bazel_wrapper::bep::build_events::hydrated_stream::{HydratedInfo, HydratorState};
use bazelfe_protos::build_event_stream::BuildEvent;
use clap::Parser;
use prost::Message;
use std::collections::VecDeque;
use std::error::Error;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[clap(name = "bep-junit")]
struct Opt {
    #[clap(long)]
    build_event_binary_output: PathBuf,

    #[clap(long)]
    junit_output_path: PathBuf,
}

fn load_build_event_proto(
    d: &Path,
) -> Result<impl Iterator<Item = Result<BuildEvent, Box<dyn Error>>>, Box<dyn Error>> {
    struct IterC(VecDeque<u8>);
    impl Iterator for IterC {
        type Item = Result<BuildEvent, Box<dyn Error>>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                None
            } else {
                let decoded = BuildEvent::decode_length_delimited(&mut self.0);
                let right_err =
                    decoded.map_err(|de| Box::new(Into::<std::io::Error>::into(de)).into());
                Some(right_err)
            }
        }
    }

    let mut file = std::fs::File::open(d)?;

    let mut data_vec = VecDeque::default();
    std::io::copy(&mut file, &mut data_vec)?;

    Ok(IterC(data_vec))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    std::fs::create_dir_all(&opt.junit_output_path)?;

    let result_protos = load_build_event_proto(opt.build_event_binary_output.as_path())?;

    let mut hydrator = HydratorState::default();
    let mut hydrated_infos = Vec::default();
    // we use a for loop so we can use .? which gets complex with a map/flat_map
    for result_build_event in result_protos {
        hydrated_infos.extend(
            &mut hydrator
                .consume(BuildEventAction::BuildEvent(result_build_event?.into()))
                .into_iter()
                .flatten(),
        );
    }
    hydrated_infos.extend(
        &mut hydrator
            .consume(BuildEventAction::BuildCompleted)
            .into_iter()
            .flatten(),
    );

    let mut failed_actions = Vec::default();
    let mut aborted_actions = Vec::default();
    let mut failed_tests = Vec::default();
    let mut failed_xml_writes = Vec::default();
    for build_event in hydrated_infos.into_iter() {
        match &build_event {
            HydratedInfo::BazelAbort(abort_info) => {
                aborted_actions.push(abort_info.label.clone());
                match emit_junit_xml_from_aborted_action(
                    &abort_info,
                    aborted_actions.len(),
                    &opt.junit_output_path,
                ) {
                    Ok(_) => (),
                    Err(e) => failed_xml_writes.push((build_event, e)),
                }
            }
            HydratedInfo::ActionFailed(action_failed) => {
                failed_actions.push(action_failed.label.clone());
                match emit_junit_xml_from_failed_action(&action_failed, &opt.junit_output_path) {
                    Ok(_) => (),
                    Err(e) => failed_xml_writes.push((build_event, e)),
                }
            }
            HydratedInfo::TestResult(r) => {
                let is_failure = r.test_summary_event.test_status.didnt_pass();
                if is_failure {
                    failed_tests.push(r.test_summary_event.label.clone());
                }
                let output_folder = opt.junit_output_path.join(label_to_junit_relative_path(
                    r.test_summary_event.label.as_str(),
                ));

                match std::fs::create_dir_all(&output_folder) {
                    Ok(_) => {
                        let files: Vec<&str> = r
                            .test_summary_event
                            .output_files
                            .iter()
                            .flat_map(|e| match e {
                                bazelfe_protos::build_event_stream::file::File::Uri(uri) => {
                                    let p = uri.strip_prefix("file://").unwrap_or_else(|| {
                                        panic!("Wasn't a local file for {}", uri)
                                    });
                                    if p.ends_with("/test.xml") {
                                        Some(p)
                                    } else {
                                        None
                                    }
                                }
                                bazelfe_protos::build_event_stream::file::File::Contents(_) => None,
                            })
                            .collect();

                        for (idx, f) in files.into_iter().enumerate() {
                            match std::fs::metadata(f) {
                          Ok(m) => {
                            if m.size() > 0 {
                              let output_file = output_folder.join(format!("test.{}.xml", idx));
                              match std::fs::copy(f, output_file) {
                                Ok(_) => (),
                                Err(e) =>
                                  println!("could not access metadata for test result {} at file {}.\nError {}",
                                    r.test_summary_event.label,
                                    f,
                                    e)
                              }
                            }
                          }
                          Err(e) =>
                            println!("could not access metadata for test result {} at file {}.\nError {}",
                              r.test_summary_event.label,
                              f,
                              e)
                        }
                        }
                        if is_failure {
                            // Some failures don't get to the phase of writing junit output
                            // this ensures we write something
                            match emit_backup_error_data(&r, &opt.junit_output_path) {
                                Ok(_) => (),
                                Err(err) => failed_xml_writes.push((build_event, err)),
                            }
                        }
                    }
                    Err(e) => failed_xml_writes.push((build_event, e.into())),
                }
            }
            HydratedInfo::Progress(_) => (),
            HydratedInfo::ActionSuccess(_) => (),
            HydratedInfo::TargetComplete(_) => (),
        }
    }

    if failed_actions.is_empty()
        && failed_tests.is_empty()
        && aborted_actions.is_empty()
        && failed_xml_writes.is_empty()
    {
        println!("Have zero failures, all successful.");
        Ok(())
    } else {
        if !failed_actions.is_empty() {
            println!("Have {} failed actions", failed_actions.len());
            for a in failed_actions {
                println!("  - {}", a);
            }
        }

        if !failed_tests.is_empty() {
            println!("Have {} failed tests", failed_tests.len());
            failed_tests.sort();
            for a in failed_tests {
                println!("  - {}", a);
            }
        }

        if !aborted_actions.is_empty() {
            println!("Have {} aborted actions", aborted_actions.len());
            aborted_actions.sort();
            for a in aborted_actions {
                println!("  - {}", a.unwrap_or_else(|| "Unknown".to_string()));
            }
        }

        if !failed_xml_writes.is_empty() {
            println!("Got {} xml write failures", failed_xml_writes.len());
            failed_xml_writes.sort_by(|p1, p2| p1.0.label().cmp(&p2.0.label()));
            for (r, err) in failed_xml_writes {
                println!(
                    "Target label = {} failed to write: {}",
                    r.label().unwrap_or("<unknown>"),
                    err
                );
            }
        }

        let err: Box<dyn Error> = Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "non-zero error count.",
        ));
        Err(err)
    }
}
