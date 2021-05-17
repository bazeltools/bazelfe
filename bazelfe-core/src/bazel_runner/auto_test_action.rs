// use crate::{buildozer_driver, config::command_line_rewriter::TestActionMode};
// use crate::config::CommandLineRewriter;

// use thiserror::Error;

// use super::configured_bazel_runner::ConfiguredBazelRunner;

// #[derive(Error, Debug)]
// pub enum RewriteCommandLineError {
//     #[error("Reporting user error: `{0}`")]
//     UserErrorReport(super::UserReportError),
// }

// fn find_first_non_flag_arg<'a>(iter: impl Iterator<Item = &'a String>) -> Option<(usize, String)> {
//     iter.enumerate().find_map(|(idx, e)| {
//         if e.starts_with("--") {
//             None
//         } else {
//             Some((idx, e.clone()))
//         }
//     })
// }

// pub async fn maybe_auto_test_mode<
//     T: buildozer_driver::Buildozer,
//     U: crate::hydrated_stream_processors::process_bazel_failures::CommandLineRunner,
// >(
//     configured_bazel_runner: &mut ConfiguredBazelRunner<T, U>
// ) -> Result<bool, Box<dyn std::error::Error>> {
//     // Keep in mind here the first arg is the path to the bazel binary, so needs to be ignored!

//     let action_and_pos = find_first_non_flag_arg(args.iter().skip(1));

//     if let Some((indx, action)) = action_and_pos {
//         if action != "autotest" {
//             return Ok(false);
//         }

//         eprintln!("Entering auto-test mode");

//             let iter = args.iter();
//             // we ignore the error here since if there are no more args left the iterator will return None
//             // and we are safely done anyway. We just needed to skip beyond the current arg.(also skip the bazel binary at the start.)
//             let iter = iter.skip(indx + 2);
//             let target_opt = find_first_non_flag_arg(iter);

//             if target_opt.is_none() {
//                 match &command_line_rewriter.test {
//                     TestActionMode::EmptyTestToLocalRepo(cfg) => {
//                         args.push(cfg.command_to_use.clone());
//                     }
//                     TestActionMode::EmptyTestToFail => {
//                         Err(RewriteCommandLineError::UserErrorReport(super::UserReportError("No test target specified.\nUnlike other build tools, bazel requires you specify which test target to test.\nTo test the whole repo add //... to the end. But beware this could be slow!".to_owned())))?;
//                     }
//                     TestActionMode::Passthrough => {}
//                     TestActionMode::SuggestTestTarget(cfg) => {
//                         if let Some(daemon_cli) = daemon_client.as_ref() {
//                             let mut invalidated_targets = vec![];

//                             for distance in 0..(cfg.distance_to_expand + 1) {
//                                 let recently_invalidated_targets = daemon_cli
//                                     .recently_invalidated_targets(
//                                         tarpc::context::current(),
//                                         distance,
//                                     )
//                                     .await;
//                                 invalidated_targets.extend(
//                                     recently_invalidated_targets
//                                         .into_iter()
//                                         .map(|e| (distance, e)),
//                                 );
//                             }
//                             if !invalidated_targets.is_empty() {
//                                 use trim_margin::MarginTrimmable;

//                                 invalidated_targets.sort_by_key(|e| e.0);
//                                 use std::collections::HashSet;
//                                 let mut seen_targets: HashSet<String> = HashSet::default();
//                                 let mut buf = String::from("");
//                                 invalidated_targets.into_iter().for_each(|(_, targets)| {
//                                     targets.iter().for_each(|target| {
//                                         if target.is_test() {
//                                             if !seen_targets.contains(target.target_label()) {
//                                                 seen_targets.insert(target.target_label().clone());
//                                                 buf.push_str(&format!(
//                                                     "\n|{}",
//                                                     target.target_label()
//                                                 ));
//                                             }
//                                         }
//                                     });
//                                 });

//                                 Err(RewriteCommandLineError::UserErrorReport(
//                                     super::UserReportError(
//                                         format!(
//                                             r#"|No test target specified.
//                                     | Suggestions:
//                                     |{}
//                                     |"#,
//                                             buf
//                                         )
//                                         .trim_margin()
//                                         .unwrap()
//                                         .to_owned(),
//                                     ),
//                                 ))?;
//                             }
//                         } else {
//                             Err(RewriteCommandLineError::UserErrorReport(super::UserReportError(
//                                 "Configured to suggest possible test targets to run, but no daemon is running".to_owned())))?;
//                         }
//                     }
//                 }
//             }
//         }
//     }

//     Ok(())
// }
