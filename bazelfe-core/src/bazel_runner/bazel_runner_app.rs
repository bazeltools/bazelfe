use clap::{AppSettings, Clap};
use std::path::PathBuf;

use std::ffi::OsString;

use bazelfe_core::bazel_runner;
use bazelfe_core::config::Config;

use std::sync::Arc;

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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(_) = std::env::var("BAZEL_FE_ENABLE_DAEMON_MODE") {
        return Ok(bazelfe_core::bazel_runner_daemon::daemon_server::base_main().await?);
    }

    let opt = Opt::parse();

    // If someone is using a bes backend we need to nope out so we don't conflict.
    // This also means our other tools can call in using our same utilities
    // with this already set to make this app passthrough
    if opt
        .passthrough_args
        .contains(&String::from("--bes_backend"))
    {
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

    let config = Arc::new(config);
    let bazel_runner = bazel_runner::bazel_runner::BazelRunner {
        config: Arc::clone(&config),
        passthrough_args: opt.passthrough_args.clone(),
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
