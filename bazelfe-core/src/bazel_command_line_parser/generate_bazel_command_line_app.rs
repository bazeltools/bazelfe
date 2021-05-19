use clap::Clap;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use tokio::process::Command;
#[derive(Clap, Debug)]
#[clap(name = "basic")]
struct Opt {
    #[clap(long, parse(from_os_str))]
    bazel_cmd_path: PathBuf,

    #[clap(parse(from_os_str))]
    output_path: PathBuf,
}
fn decode_str(data: &Vec<u8>) -> Result<String, Box<dyn Error>> {
    if data.len() > 0 {
        Ok(std::str::from_utf8(&data)?.to_string())
    } else {
        Ok(String::from(""))
    }
}

async fn generate_actions_list(bazel_path: &PathBuf) -> Result<Vec<String>, Box<dyn Error>> {
    let mut res = Vec::default();

    let mut command = Command::new(bazel_path);

    command.arg("help");
    let output = command.output().await?;

    let stdout = decode_str(&output.stdout)?;

    let mut in_segment = false;
    for line in stdout.lines() {
        if in_segment {
            if line.trim().is_empty() {
                break;
            } else {
                let segment = line.split(" ").filter(|e| !e.is_empty()).next();
                if let Some(action) = segment {
                    res.push(action.to_string());
                } else {
                    panic!("Unable to parse line {}", line);
                }
            }
        } else {
            if line.starts_with("Available commands:") {
                in_segment = true;
            }
        }
    }

    Ok(res)
}

#[derive(Debug, Clone, Ord, PartialEq, Eq, PartialOrd, Hash)]
enum BazelOption {
    BooleanOption(String),
    OptionWithArg(String),
}

fn valid_option(opt: &str) -> bool {
    if opt.is_empty() {
        return false;
    }
    for chr in opt.chars() {
        match chr {
            'a'..='z' => (),
            '_' => (),
            _ => {
                return false;
            }
        }
    }
    true
}
async fn extract_options(
    bazel_path: &PathBuf,
    action: &str,
) -> Result<Vec<BazelOption>, Box<dyn Error>> {
    let mut res = Vec::default();

    let mut command = Command::new(bazel_path);

    command.args(&["help", action, "--short"]);

    eprintln!("Running {:#?}", command);
    let output = command.output().await?;

    let stdout = decode_str(&output.stdout)?;

    for line in stdout.lines() {
        let segment = line.split(" ").filter(|e| !e.is_empty()).next();
        if let Some(action) = segment {
            if action.starts_with("--") {
                if action.starts_with("--[no]") {
                    let opt = &action[6..];
                    if valid_option(opt) {
                        res.push(BazelOption::BooleanOption(opt.to_string()));
                    }
                } else {
                    let opt = &action[2..];
                    if valid_option(opt) {
                        res.push(BazelOption::OptionWithArg(opt.to_string()));
                    }
                }
            }
        }
    }

    Ok(res)
}
fn convert_raw_action_to_enum_name(s: &str) -> String {
    let mut v: Vec<char> = Vec::default();
    let mut last_special = true;
    for chr in s.chars() {
        if last_special {
            let mut chr = chr.clone();
            chr.make_ascii_uppercase();
            v.push(chr);
            last_special = false;
        } else {
            match chr {
                'a'..='z' => {
                    v.push(chr);
                }
                _ => {
                    last_special = true;
                }
            }
        }
    }
    v.iter().collect()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::parse();

    let mut actions = generate_actions_list(&opt.bazel_cmd_path).await?;
    actions.sort();
    println!("{:?}", actions);

    let startup_options = extract_options(&opt.bazel_cmd_path, "startup_options").await?;

    let mut options_per_action: HashMap<String, Vec<BazelOption>> = HashMap::default();
    for action in actions.iter() {
        let options = extract_options(&opt.bazel_cmd_path, &action).await?;
        options_per_action.insert(action.clone(), options);
    }

    let mut all_action_args: Vec<BazelOption> = options_per_action
        .iter()
        .flat_map(|(_, v)| v.iter())
        .cloned()
        .collect();

    all_action_args.sort();
    all_action_args.dedup();

    let mut action_lookup: HashMap<BazelOption, usize> = HashMap::default();

    for (idx, opt) in all_action_args.iter().enumerate() {
        action_lookup.insert(opt.clone(), idx);
    }

    use std::fs::File;
    use std::io::prelude::*;

    let mut file = File::create(&opt.output_path)?;
    file.write_all(b"use super::*;\n")?;
    file.write_all(b"use lazy_static::lazy_static;\n")?;

    // First Write out all the action options.
    file.write_all(b"lazy_static! {\n")?;
    file.write_all(b"    pub static ref ALL_ACTION_OPTIONS: Vec<BazelOption> = {\n")?;
    file.write_all(b"        let mut vec = Vec::new();\n")?;

    for action_arg in all_action_args.iter() {
        match action_arg {
            BazelOption::BooleanOption(bool_option) => {
                file.write_all(
                    format!(
                        r#"    vec.push(BazelOption::BooleanOption(String::from("{}"), false));"#,
                        bool_option
                    )
                    .as_bytes(),
                )?;
            }
            BazelOption::OptionWithArg(with_arg_option) => {
                file.write_all(
                    format!(
                        r#"    vec.push(BazelOption::OptionWithArg(String::from("{}"), String::default()));"#,
                        with_arg_option
                    )
                    .as_bytes(),
                )?;
            }
        }
        file.write_all(b"\n")?;
    }
    file.write_all(b"        vec\n")?;
    file.write_all(b"    };\n")?;
    file.write_all(b"}\n")?;
    file.write_all(b"\n")?;
    file.write_all(b"\n")?;

    file.write_all(b"lazy_static! {\n")?;
    file.write_all(b"    pub static ref STARTUP_OPTIONS: Vec<BazelOption> = {\n")?;
    file.write_all(b"        let mut vec = Vec::new();\n")?;

    for action_arg in startup_options.iter() {
        match action_arg {
            BazelOption::BooleanOption(bool_option) => {
                file.write_all(
                    format!(
                        r#"    vec.push(BazelOption::BooleanOption(String::from("{}"), false));"#,
                        bool_option
                    )
                    .as_bytes(),
                )?;
            }
            BazelOption::OptionWithArg(with_arg_option) => {
                file.write_all(
                    format!(
                        r#"    vec.push(BazelOption::OptionWithArg(String::from("{}"), String::default()));"#,
                        with_arg_option
                    )
                    .as_bytes(),
                )?;
            }
        }
        file.write_all(b"\n")?;
    }
    file.write_all(b"        vec\n")?;
    file.write_all(b"    };\n")?;
    file.write_all(b"}\n")?;
    file.write_all(b"\n")?;
    file.write_all(b"\n")?;

    file.write_all(b"lazy_static! {\n")?;
    file.write_all(b"    pub static ref ACTION_TO_OPTIONS: std::collections::HashMap<BuiltInAction, Vec<usize>> = {\n")?;
    file.write_all(b"        let mut map = std::collections::HashMap::new();\n")?;

    for action in actions.iter() {
        let action_title = convert_raw_action_to_enum_name(action);

        let options = options_per_action
            .get(action)
            .expect("Should have options for this action");

        let mut options_indices: Vec<usize> = options
            .iter()
            .map(|opt| action_lookup.get(opt).expect("Should exist in table"))
            .cloned()
            .collect();
        options_indices.sort();
        let mut str_buf: String = String::default();
        for u in options_indices {
            str_buf = format!("{}{},", str_buf, u);
        }
        file.write_all(
            format!(
                "        map.insert(BuiltInAction::{}, vec![{}]);\n",
                action_title, str_buf
            )
            .as_bytes(),
        )?;
    }
    file.write_all(b"        map\n")?;
    file.write_all(b"    };\n")?;
    file.write_all(b"}\n")?;
    file.write_all(b"\n")?;
    file.write_all(b"\n")?;

    file.write_all(b"#[derive(Debug, Clone, PartialEq, Eq, Hash)]\n")?;
    file.write_all(b"pub enum BuiltInAction {\n")?;

    for action in actions.iter() {
        let action_title = convert_raw_action_to_enum_name(action);
        file.write_all(format!("    {},\n", action_title).as_bytes())?;
    }
    file.write_all(b"}\n")?;

    file.write_all(b"\n")?;
    file.write_all(b"use std::str::FromStr;\n")?;
    file.write_all(b"impl FromStr for BuiltInAction {\n")?;
    file.write_all(b"\n")?;
    file.write_all(b"    type Err = ();\n")?;
    file.write_all(b"\n")?;
    file.write_all(b"    fn from_str(input: &str) -> Result<BuiltInAction, Self::Err> {\n")?;
    file.write_all(b"        match input {\n")?;
    for action in actions.iter() {
        let action_title = convert_raw_action_to_enum_name(action);
        file.write_all(
            format!(
                "            \"{}\"  => Ok(BuiltInAction::{}),\n",
                action, action_title
            )
            .as_bytes(),
        )?;
    }
    file.write_all(b"            _ => Err(()),\n")?;
    file.write_all(b"        }\n")?;
    file.write_all(b"    }\n")?;
    file.write_all(b"}\n")?;

    file.write_all(b"impl core::fmt::Display for BuiltInAction {\n")?;
    file.write_all(b"\n")?;

    file.write_all(b"    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n")?;
    file.write_all(b"        match self {\n")?;
    for action in actions.iter() {
        let action_title = convert_raw_action_to_enum_name(action);
        file.write_all(
            format!(
                "            BuiltInAction::{}  => Ok(write!(f, \"{}\")?),\n",
                action_title, action,
            )
            .as_bytes(),
        )?;
    }
    file.write_all(b"        }\n")?;
    file.write_all(b"    }\n")?;
    file.write_all(b"}\n")?;

    Ok(())
}
