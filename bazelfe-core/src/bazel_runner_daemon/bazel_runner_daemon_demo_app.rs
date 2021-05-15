use bazelfe_core::config::DaemonConfig;
use clap::Clap;
use std::error::Error;
use std::path::PathBuf;

#[derive(PartialEq, Eq, Debug)]
enum RunMode {
    SpawnDaemon,
    QueryGraph,
}
impl std::str::FromStr for RunMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "spawn-daemon" {
            Ok(RunMode::SpawnDaemon)
        } else if s == "query-graph" {
            Ok(RunMode::QueryGraph)
        } else {
            Err(String::from("Unknown option mode"))
        }
    }
}

#[derive(Clap, Debug)]
enum SubCommands {
    SpawnDaemon,
    QueryGraph,
    RunForkExecDaemon(DaemonArgs),
}

#[derive(Clap, Debug)]
struct DaemonArgs {
    #[clap(parse(from_os_str))]
    pub config_path: PathBuf,
}

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    #[clap(long, parse(from_os_str))]
    bazel_binary_path: Option<PathBuf>,

    #[clap(subcommand)]
    subcmd: SubCommands,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Ok(_) = std::env::var("BAZEL_FE_ENABLE_DAEMON_MODE") {
        return Ok(bazelfe_core::bazel_runner_daemon::daemon_server::base_main().await?);
    }

    let opt = Opt::parse();

    match opt.subcmd {
        SubCommands::SpawnDaemon => {
            let mut daemon_config = DaemonConfig::default();

            daemon_config.enabled = true;

            let bazel_path = opt
                .bazel_binary_path
                .expect("Should have a bazel binary path when running query.");
            bazelfe_core::bazel_runner_daemon::daemon_manager::connect_to_server(
                &daemon_config,
                &bazel_path,
            )
            .await?;

            // match bazelfe_core::bazel_runner_daemon::spawn_daemon(&PathBuf::from("/tmp/daemon_talk"), &PathBuf::from("/tmp/daemon_talk/daemon.pid"))? {
            //     bazelfe_core::bazel_runner_daemon::DaemonType::CommandLineCli => {
            //         eprintln!("Command line client wing...{}", std::process::id());
            //     }
            //     bazelfe_core::bazel_runner_daemon::DaemonType::DaemonProcess => {
            //         let current_dir =
            //             std::env::current_dir().expect("Failed to determine current directory");

            //         let current_dir = current_dir;

            //         use notify::{RecommendedWatcher, RecursiveMode, Result, Watcher};
            //         use std::time::Duration;

            //         let mut watcher: RecommendedWatcher = Watcher::new_immediate(|res| match res {
            //             Ok(event) => println!("event: {:?}", event),
            //             Err(e) => println!("watch error: {:?}", e),
            //         })
            //         .unwrap();

            //         // Add a path to be watched. All files and directories at that path and
            //         // below will be monitored for changes.
            //         watcher
            //             .watch(current_dir, RecursiveMode::Recursive)
            //             .unwrap();

            //         std::thread::sleep(std::time::Duration::from_secs(120));

            //         eprintln!("DaemonProcess wing...{}, quitting.", std::process::id());
            //         println!("DaemonProcess wing...{}, quitting.", std::process::id());
            //         std::process::exit(0);
            //     }
            // }
        }
        SubCommands::QueryGraph => {
            let bazel_query = bazelfe_core::jvm_indexer::bazel_query::from_binary_path(
                &opt.bazel_binary_path
                    .expect("Should have a bazel binary path when running query."),
            );

            bazelfe_core::bazel_runner_daemon::query_graph::graph_query(
                &bazel_query,
                "deps(//...)",
            )
            .await?;
        }
        SubCommands::RunForkExecDaemon(daemon_args) => {
            bazelfe_core::bazel_runner_daemon::daemon_server::main_from_config(
                &daemon_args.config_path,
            )
            .await?;
        }
    }

    Ok(())
}
