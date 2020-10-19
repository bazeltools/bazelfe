use clap::Clap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use bazelfe_core::index_table::parse_file;

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    /// Files to process
    #[clap(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    for f in opt.files.iter() {
        let content = fs::read_to_string(f)?;

        let _parsed_file = parse_file(&content).unwrap();
    }
    Ok(())
}
