use clap::Clap;
use std::error::Error;
use std::path::PathBuf;

use bazelfe_core::buildozer_driver::{from_binary_path, Buildozer};

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    #[clap(long, env = "BUILDOZER_PATH", parse(from_os_str))]
    buildozer_path: PathBuf,

    target_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();
    let buildozer = from_binary_path(opt.buildozer_path);
    let buildozer_resp = buildozer.print_deps(&opt.target_name).await.unwrap();
    println!("{:?}", buildozer_resp);
    Ok(())
}
