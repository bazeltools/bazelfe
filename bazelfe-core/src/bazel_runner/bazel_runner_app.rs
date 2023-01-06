use clap::Parser;
use std::path::PathBuf;

use std::ffi::OsString;

use bazelfe_core::bazel_runner;
use bazelfe_core::bazel_runner::parse_commandline_with_custom_command_line_options;
use bazelfe_core::config::load_config_file;

#[derive(Parser, Debug)]
#[clap(name = "basic", trailing_var_arg = true)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long, env = "INDEX_INPUT_LOCATION")]
    index_input_location: Option<PathBuf>,

    #[clap(long, env = "BUILDOZER_PATH")]
    buildozer_path: PathBuf,

    #[clap(required = true, num_args = 1..)]
    passthrough_args: Vec<String>,

    #[clap(long, env = "DISABLE_ACTION_STORIES_ON_SUCCESS")]
    disable_action_stories_on_success: bool,

    #[clap(long)]
    config: Option<String>,

    #[clap(long)]
    validate_index_file: Option<PathBuf>,
}

fn passthrough_to_bazel(opt: Opt) {
    let application: OsString = opt
        .passthrough_args
        .first()
        .map(|a| {
            let a: String = a.clone();
            a
        })
        .expect("Should have had at least one arg the bazel process itself.")
        .into();

    let remaining_args: Vec<OsString> = opt
        .passthrough_args
        .iter()
        .skip(1)
        .map(|str_ref| {
            let a: String = str_ref.clone();
            let a: OsString = a.into();
            a
        })
        .collect();

    let resp = ::exec::Command::new(application)
        .args(&remaining_args)
        .exec();
    panic!("Should be unreachable: {:#?}", resp);
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "bazelfe-daemon")]
    if std::env::var("BAZEL_FE_ENABLE_DAEMON_MODE").is_ok() {
        return bazelfe_core::bazel_runner_daemon::daemon_server::base_main().await;
    }

    let opt = Opt::parse();

    if let Some(index_file_path) = &opt.validate_index_file {
        let mut src_f = std::fs::File::open(index_file_path.clone()).unwrap();
        return match bazelfe_core::index_table::IndexTable::read(&mut src_f) {
            Ok(_) => Ok(()),
            Err(e) => Err(format!(
                "Failed to parse index file @ {:?}, error\n:{:?}",
                index_file_path, e
            )
            .into()),
        };
    }
    // TODO IN HERE,
    // fixed actions we intercept
    // bail of a bes_backend is already configured.
    let parsed_command_line = match parse_commandline_with_custom_command_line_options(&opt.passthrough_args) {
        Ok(parsed_command_line) => {
            if parsed_command_line.is_action_option_set("bes_backend") {
                // Likely tooling is setting this, quietly exec bazel.
                // since we can't invoke our usual behaviors if this is the case.
                // Need to figure out some way to signal this occured probably to the dev productivity team somewhere.
                return {
                    passthrough_to_bazel(opt);
                    Ok(())
                };
            }
            parsed_command_line
        }
        Err(cmd_line_parsing_failed) => {
            match cmd_line_parsing_failed {
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::MissingBazelPath => {
                    eprintln!("Missing bazel path, invalid command line arg supplied");
                    std::process::exit(-1);
                }
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::MissingArgToOption(o) => {
                        eprintln!("Arg parsing from bazelfe doesn't understand the args, missing an option to {}", o);
                        eprintln!("Will just invoke bazel and abort.");
                        return {
                            passthrough_to_bazel(opt);
                            Ok(())
                        };
                }
                bazelfe_bazel_wrapper::bazel_command_line_parser::CommandLineParsingError::UnknownArgument(o) => {
                    eprintln!("We got an option we didn't know how to parse, to avoid doing something unexpected, we will just invoke bazel.\nGot: {}", o);
                    return {
                        passthrough_to_bazel(opt);
                        Ok(())
                    };
                }
            }
        }
    };

    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    builder.target(pretty_env_logger::env_logger::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        let f = if s.contains("tarpc") {
            s
        } else {
            format!("tarpc::client=error,{}", s)
        };
        builder.parse_filters(&f);
    } else {
        builder.parse_filters("warn,tarpc::client=error,bazelfe_core=info,bazel_runner=info");
    }
    builder.init();

    let mut config = load_config_file(&opt.config.as_ref()).await?;

    config.buildozer_path = Some(opt.buildozer_path);

    if opt.index_input_location.is_some() {
        config.index_input_location = opt.index_input_location;
    }

    if let Some(addr) = opt.bind_address {
        let addr: std::net::SocketAddr = addr.parse()?;
        config.bes_server_bind_address = Some(addr);
    }

    if opt.disable_action_stories_on_success {
        config.disable_action_stories_on_success = opt.disable_action_stories_on_success;
    }

    let bazel_runner = bazel_runner::BazelRunner {
        config,
        bazel_command_line: parsed_command_line,
    };

    match bazel_runner.run().await {
        Ok(final_exit_code) => {
            std::process::exit(final_exit_code);
        }
        Err(ex) => {
            match ex {
                bazel_runner::BazelRunnerError::UserErrorReport(user_error) => {
                    eprintln!("\x1b[0;31m{}\x1b[0m", user_error.0);
                }
                other => eprintln!("Error:\n{}", other),
            }
            std::process::exit(-1);
        }
    }
}
