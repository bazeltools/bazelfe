use std::error::Error;
use std::path::PathBuf;
use bazelfe_core::jvm_indexer::bazel_query::BazelQuery;
use bazelfe_protos::build::bazel;
use clap::Clap;


#[derive(PartialEq, Eq, Debug)]
enum RunMode {
    SpawnDaemon,
    QueryGraph
}
impl std::str::FromStr for RunMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "spawn-daemon" {
            Ok(RunMode::SpawnDaemon)
        }else if s == "query-graph" {
            Ok(RunMode::QueryGraph)
        } else {
            Err(String::from("Unknown option mode"))
        }
    }
}

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    mode: RunMode,

    #[clap(long, parse(from_os_str))]
    bazel_binary_path: Option<PathBuf>,

}



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {


    let opt = Opt::parse();


    if opt.mode == RunMode::SpawnDaemon {
    match bazelfe_core::bazel_runner_daemon::spawn_daemon(PathBuf::from("/tmp/daemon_talk"))? {
        bazelfe_core::bazel_runner_daemon::DaemonType::CommandLineCli => {
            eprintln!("Command line client wing...{}", std::process::id());
        }
        bazelfe_core::bazel_runner_daemon::DaemonType::DaemonProcess => {
            std::thread::sleep(std::time::Duration::from_secs(2));
            eprintln!("DaemonProcess wing...{}", std::process::id());
        }
    }
} else if opt.mode == RunMode::QueryGraph {

    let bazel_query =
    bazelfe_core::jvm_indexer::bazel_query::from_binary_path(opt.bazel_binary_path.expect("Should have a bazel binary path when running query."));

    bazelfe_core::bazel_runner_daemon::query_graph::graph_query(&bazel_query, "deps(//...)").await;

} else  {
    eprintln!("Unknown mode, no-op");
}
    Ok(())
}
