use clap::{AppSettings, Clap};
use std::path::PathBuf;

use std::ffi::OsString;

use bazelfe_core::config::Config;
use bazelfe_core::{bazel_command_line_parser::parse_bazel_command_line, bazel_runner};

#[derive(Clap, Debug)]
#[clap(name = "basic", setting = AppSettings::TrailingVarArg)]
struct Opt {
    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long, env = "INDEX_INPUT_LOCATION", parse(from_os_str))]
    index_input_location: Option<PathBuf>,

    #[clap(long, env = "BUILDOZER_PATH", parse(from_os_str))]
    buildozer_path: PathBuf,

    #[clap(required = true, min_values = 1)]
    passthrough_args: Vec<String>,

    #[clap(long, env = "DISABLE_ACTION_STORIES_ON_SUCCESS")]
    disable_action_stories_on_success: bool,

    #[clap(long)]
    config: Option<String>,
}

async fn load_config_file(opt: &Opt) -> Result<Config, Box<dyn std::error::Error>> {
    use std::str::FromStr;
    let mut path: Option<String> = None;
    if let Some(p) = &opt.config {
        let pbuf = PathBuf::from_str(&p)?;
        if !pbuf.exists() {
            panic!("Expected to find config at path {}, but it didn't exist", p);
        }
        path = Some(p.clone())
    };

    if path == None {
        if let Ok(home_dir) = std::env::var("HOME") {
            let cur_p = PathBuf::from(format!("{}/.bazelfe_config", home_dir));
            if cur_p.exists() {
                path = Some(cur_p.to_str().unwrap().to_string());
            }
        }
    }

    if let Some(path) = path {
        Ok(bazelfe_core::config::parse_config(
            &std::fs::read_to_string(path)?,
        )?)
    } else {
        Ok(Config::default())
    }
}

fn passthrough_to_bazel(opt: Opt) -> () {
    let application: OsString = opt
        .passthrough_args
        .first()
        .map(|a| {
            let a: String = a.clone().into();
            a
        })
        .expect("Should have had at least one arg the bazel process itself.")
        .into();

    let remaining_args: Vec<OsString> = opt
        .passthrough_args
        .iter()
        .skip(1)
        .map(|str_ref| {
            let a: String = str_ref.clone().into();
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
    if let Ok(_) = std::env::var("BAZEL_FE_ENABLE_DAEMON_MODE") {
        return Ok(bazelfe_core::bazel_runner_daemon::daemon_server::base_main().await?);
    }

    let opt = Opt::parse();

    // TODO IN HERE,
    // fixed actions we intercept
    // bail of a bes_backend is already configured.
    let parsed_command_line = match parse_bazel_command_line(&opt.passthrough_args) {
        Ok(parsed_command_line) => {
            if parsed_command_line.is_action_option_set("bes_backend") {
                // Likely tooling is setting this, quietly exec bazel.
                // since we can't invoke our usual behaviors if this is the case.
                // Need to figure out some way to signal this occured probably to the dev productivity team somewhere.
                return Ok(passthrough_to_bazel(opt));
            }
            parsed_command_line
        }
        Err(cmd_line_parsing_failed) => {
            match cmd_line_parsing_failed {
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::MissingBazelPath => {
                    eprintln!("Missing bazel path, invalid command line arg supplied");
                    std::process::exit(-1);
                }
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::MissingArgToOption(o) => {
                        eprintln!("Arg parsing from bazelfe doesn't understand the args, missing an option to {}", o);
                        eprintln!("Will just invoke bazel and abort.");
                        return Ok(passthrough_to_bazel(opt));
                }
                bazelfe_core::bazel_command_line_parser::CommandLineParsingError::UnknownArgument(o) => {
                    eprintln!("We got an option we didn't know how to parse, to avoid doing something unexpected, we will just invoke bazel.\nGot: {}", o);
                    return Ok(passthrough_to_bazel(opt));
                }
            }
        }
    };

    let mut builder = pretty_env_logger::formatted_timed_builder();
    builder.format_timestamp_nanos();
    builder.target(pretty_env_logger::env_logger::Target::Stderr);
    if let Ok(s) = ::std::env::var("RUST_LOG") {
        builder.parse_filters(&s);
    } else {
        builder.parse_filters("warn,bazelfe_core=info,bazel_runner=info");
    }
    builder.init();

    let mut config = load_config_file(&opt).await?;

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

    let bazel_runner = bazel_runner::bazel_runner::BazelRunner {
        config,
        bazel_command_line: parsed_command_line,
    };

    match bazel_runner.run().await {
        Ok(final_exit_code) => {
            std::process::exit(final_exit_code);
        }
        Err(ex) => {
            match ex {
                bazel_runner::bazel_runner::BazelRunnerError::UserErrorReport(user_error) => {
                    eprintln!("\x1b[0;31m{}\x1b[0m", user_error.0);
                }
                other => eprintln!("Error:\n{}", other),
            }
            std::process::exit(-1);
        }
    }
}
