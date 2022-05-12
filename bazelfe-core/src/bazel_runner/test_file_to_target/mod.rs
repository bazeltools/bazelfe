use std::{collections::HashSet, path::PathBuf};

use crate::{
    bazel_command_line_parser::{BuiltInAction, ParsedCommandLine},
    jvm_indexer::bazel_query::BazelQuery,
};

use super::command_line_rewriter_action::RewriteCommandLineError;

fn err(e_string: String) -> Result<(), RewriteCommandLineError> {
    Err(RewriteCommandLineError::UserErrorReport(
        super::UserReportError(e_string),
    ))
}
pub async fn run<B: BazelQuery>(
    command_line: &mut ParsedCommandLine,
    replace_action: BuiltInAction,
    bazel_query: B,
) -> Result<(), RewriteCommandLineError> {
    let mut on_disk_files = Vec::default();
    for command_line_opts in command_line.remaining_args.iter() {
        let pb = PathBuf::from(command_line_opts);
        if !pb.exists() {
            return err(format!(
                "Path specified {}, does not exist",
                pb.to_string_lossy()
            ));
        }
        if !pb.is_relative() {
            return err(format!("Path specified {}, is not relative to the repo, absolute paths are not yet supported.", pb.to_string_lossy()));
        }
        on_disk_files.push(pb);
    }

    if on_disk_files.is_empty() {
        return err(format!("No files on disk specified to run"));
    }

    let mut targets: HashSet<String> = HashSet::default();

    for p in on_disk_files.into_iter() {
        let mut cur_parent = p.parent();
        while let Some(parent) = cur_parent {
            if parent.join("BUILD.bazel").exists() || parent.join("BUILD").exists() {
                break;
            }
            cur_parent = parent.parent();
        }
        let root_with_build = if let Some(root_with_build) = cur_parent {
            root_with_build.to_path_buf()
        } else {
            return err(format!(
                "Couldn't find a parent with a BUILD or BUILD.bazel in any parent of {}",
                p.to_string_lossy()
            ));
        };

        let query_for_target = bazel_query
            .execute(&vec![
                String::from("query"),
                format!(
                    "rdeps(//{}/..., {},1)",
                    root_with_build.to_string_lossy(),
                    p.to_string_lossy()
                ),
            ])
            .await;
        if query_for_target.exit_code != 0 {
            return err(format!("Attempted to query for target that owns the file {}, but bazel query returned error:\n{}", p.to_string_lossy(), query_for_target.stderr));
        }

        if let Some(ln) = query_for_target.stdout.lines().nth(1) {
            targets.insert(ln.to_string());
        }
    }

    if targets.is_empty() {
        return err(format!(
            "Attempted to query for targets built failed to find any, looked for files:\n{}",
            command_line.remaining_args.join(",")
        ));
    }

    command_line.remaining_args.clear();
    command_line.remaining_args.extend(targets.into_iter());

    command_line.action = Some(crate::bazel_command_line_parser::Action::BuiltIn(
        replace_action,
    ));
    Ok(())
}
