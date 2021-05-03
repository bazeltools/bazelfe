use crate::config::command_line_rewriter::TestActionMode;
use crate::config::CommandLineRewriter;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RewriteCommandLineError {
    #[error("Reporting user error: `{0}`")]
    UserErrorReport(super::UserReportError),
}

fn find_first_non_flag_arg<'a>(iter: impl Iterator<Item = &'a String>) -> Option<(usize, String)> {
    iter.enumerate().find_map(|(idx, e)| {
        if e.starts_with("--") {
            None
        } else {
            Some((idx, e.clone()))
        }
    })
}

pub fn rewrite_command_line(
    args: &mut Vec<String>,
    command_line_rewriter: &CommandLineRewriter,
) -> Result<(), RewriteCommandLineError> {
    // Keep in mind here the first arg is the path to the bazel binary, so needs to be ignored!

    let action_and_pos = find_first_non_flag_arg(args.iter().skip(1));

    if let Some((indx, action)) = action_and_pos {
        if action == "test" {
            let iter = args.iter();
            // we ignore the error here since if there are no more args left the iterator will return None
            // and we are safely done anyway. We just needed to skip beyond the current arg.(also skip the bazel binary at the start.)
            let iter = iter.skip(indx + 2);
            let target_opt = find_first_non_flag_arg(iter);

            match &command_line_rewriter.test {
                TestActionMode::EmptyTestToLocalRepo(cfg) => {
                    if target_opt.is_none() {
                        args.push(cfg.command_to_use.clone());
                    }
                }
                TestActionMode::EmptyTestToFail => {
                    if target_opt.is_none() {
                        Err(RewriteCommandLineError::UserErrorReport(super::UserReportError("No test target specified.\nUnlike other build tools, bazel requires you specify which test target to test.\nTo test the whole repo add //... to the end. But beware this could be slow!".to_owned())))?;
                    }
                }
                TestActionMode::Passthrough => {}
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use super::*;

    use crate::config::command_line_rewriter::*;

    #[test]
    fn test_passthrough_args() {
        let mut passthrough_command_line = vec![
            "bazel".to_string(),
            "test".to_string(),
            "--foo".to_string(),
            "bar".to_string(),
        ];

        let _ = rewrite_command_line(
            &mut passthrough_command_line,
            &CommandLineRewriter::default(),
        )
        .unwrap();

        assert_eq!(
            passthrough_command_line,
            vec![
                "bazel".to_string(),
                "test".to_string(),
                "--foo".to_string(),
                "bar".to_string()
            ]
        );
    }

    #[test]
    fn test_rewrite_empty_test() {
        let mut passthrough_command_line =
            vec!["bazel".to_string(), "test".to_string(), "--foo".to_string()];

        let rewrite_config = CommandLineRewriter {
            test: TestActionMode::EmptyTestToLocalRepo(EmptyTestToLocalRepoCfg::default()),
        };
        let _ = rewrite_command_line(&mut passthrough_command_line, &rewrite_config).unwrap();

        assert_eq!(
            passthrough_command_line,
            vec![
                "bazel".to_string(),
                "test".to_string(),
                "--foo".to_string(),
                "//...".to_string()
            ]
        );
    }

    #[test]
    fn test_rewrite_empty_test_to_fail() {
        let mut passthrough_command_line =
            vec!["bazel".to_string(), "test".to_string(), "--foo".to_string()];

        let rewrite_config = CommandLineRewriter {
            test: TestActionMode::EmptyTestToFail,
        };
        let ret = rewrite_command_line(&mut passthrough_command_line, &rewrite_config);

        assert_eq!(true, ret.is_err());

        match ret {
            Ok(_) => panic!("Expected to get an error condition from the call"),
            Err(e) => match e {
                RewriteCommandLineError::UserErrorReport(err) => {
                    assert_eq!(true, err.0.contains("No test target specified"));
                }
            },
        }
    }
}
