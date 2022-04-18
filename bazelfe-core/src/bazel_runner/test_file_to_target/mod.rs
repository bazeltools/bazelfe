use std::{collections::HashSet, path::PathBuf};

use crate::{bazel_command_line_parser::ParsedCommandLine, jvm_indexer::bazel_query::BazelQuery};

use super::command_line_rewriter_action::RewriteCommandLineError;

pub async fn rewrite_test_command(
    command_line: &mut ParsedCommandLine,
) -> Result<bool, RewriteCommandLineError> {
    let mut on_disk_files = Vec::default();
    for command_line_opts in command_line.remaining_args.iter() {
        let pb = PathBuf::from(command_line_opts);
        if !pb.exists() {
            return Ok(false);
        }
        on_disk_files.push(pb);
    }

    if on_disk_files.is_empty() {
        return Ok(false);
    }

    let bazel_query = crate::jvm_indexer::bazel_query::from_binary_path(&command_line.bazel_binary);

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
            eprintln!(
                "Couldn't find a parent with a BUILD or BUILD.bazel in any parent of {}",
                p.to_string_lossy()
            );
            return Ok(false);
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
            eprintln!("Attempted to query for target that owns the file {}, but bazel query returned error:\n{}", p.to_string_lossy(), query_for_target.stderr);
            return Ok(false);
        }

        if let Some(ln) = query_for_target.stdout.lines().nth(1) {
            targets.insert(ln.to_string());
        }
    }

    if targets.is_empty() {
        eprintln!(
            "Attempted to query for targets built failed to find any, looked for files:\n{}",
            command_line.remaining_args.join(",")
        );
        return Ok(false);
    }

    eprintln!(
        "Rewritten source files: {}",
        command_line.remaining_args.join(",")
    );
    command_line.remaining_args.clear();
    command_line.remaining_args.extend(targets.into_iter());
    eprintln!("To targets: {}", command_line.remaining_args.join(","));

    Ok(false)
}
