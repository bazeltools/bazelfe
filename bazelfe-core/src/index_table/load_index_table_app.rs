use clap::Clap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

// use bazelfe_core::index_table::parse_file;

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    /// Files to process
    #[clap(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    let _ = tokio::runtime::Handle::current();

    for f in opt.files.iter() {
        let mut file = std::fs::File::open(&f).unwrap();

        let index_table = bazelfe_core::index_table::IndexTable::read(&mut file);
        println!("{:#?}", index_table);
    }
    Ok(())
}
