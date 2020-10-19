use clap::Clap;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use bazelfe_core::source_dependencies::scala::parse_file;
use bazelfe_core::source_dependencies::SelectorType;

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

        let parsed_file = parse_file(&content).unwrap();

        for import in parsed_file.imports {
            let suffix = match import.suffix {
                SelectorType::SelectorList(lst) => {
                    let arr = lst
                        .iter()
                        .map(|(a, b)| format!("{}=>{}", a, b.as_ref().unwrap_or(a)))
                        .collect::<Vec<String>>();

                    arr.join(",")
                }
                SelectorType::WildcardSelector => "*".to_string(),
                SelectorType::NoSelector => "".to_string(),
            };
            println!(
                "{}\t{}\t{}",
                f.as_path().display(),
                import.prefix_section,
                suffix
            );
        }
    }
    Ok(())
}
