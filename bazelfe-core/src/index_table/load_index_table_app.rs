use clap::Clap;
use std::path::PathBuf;
use std::{collections::HashSet, error::Error};

// use bazelfe_core::index_table::parse_file;

#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    /// Files to process
    #[clap(name = "FILE", parse(from_os_str))]
    files: Vec<PathBuf>,

    #[clap(long)]
    targets_only: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    let _ = tokio::runtime::Handle::current();

    for f in opt.files.iter() {
        let mut file = std::fs::File::open(&f).unwrap();

        let index_table = bazelfe_core::index_table::IndexTable::read(&mut file);
        let debug_table = index_table.to_debug_table().await;

        if opt.targets_only {
            let mut v: HashSet<String> = HashSet::default();
            for (_, targets) in debug_table.data_map.into_iter() {
                for (_, target) in targets {
                    v.insert(target);
                }
            }
            let mut v: Vec<String> = v.into_iter().collect();
            v.sort();
            for e in v {
                println!("{}", e);
            }
        } else {
            for (clazz, targets) in debug_table.data_map.into_iter() {
                let mut res_str = String::from("");
                for (priority, target) in targets {
                    if res_str.len() > 0 {
                        res_str = format!("{},{}:{}", res_str, priority, target);
                    } else {
                        res_str = format!("{}:{}", priority, target);
                    }
                }
                println!("{}\t{}", clazz, res_str);
            }
        }
    }
    Ok(())
}
