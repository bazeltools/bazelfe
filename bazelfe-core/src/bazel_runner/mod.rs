#[cfg(feature = "autotest-action")]
mod auto_test_action;
mod bazel_runner_inst;
pub use bazel_runner_inst::{BazelRunner, BazelRunnerError};

mod command_line_rewriter_action;
pub mod configured_bazel_runner;
mod processor_activity;
mod test_file_to_target;
pub use command_line_rewriter_action::parse_commandline_with_custom_command_line_options;
