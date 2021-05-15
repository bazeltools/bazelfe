use crate::jvm_indexer::bazel_query::BazelQuery;
use lazy_static::lazy_static;

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// bazel_query
// .execute(&vec![
//     String::from("query"),
//     String::from("--keep_going"),
//     String::from("--output"),
//     String::from("graph"),
//     query,
// ])
// .await;

fn parse_current_repo_name() -> Option<String> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r#"^\s*workspace\(\s*name\s*=\s*("|')\s*([A-Za-z0-9_-]*)("|').*$"#).unwrap();
    }

    let workspace_path = PathBuf::from("WORKSPACE");
    if workspace_path.exists() {
        let workspace_content = std::fs::read_to_string(workspace_path).unwrap();
        let ln = workspace_content
            .lines()
            .filter(|e| e.starts_with("workspace("))
            .next();
        if let Some(line) = ln {
            if let Some(captures) = RE.captures(&line) {
                return Some(String::from(captures.get(2).unwrap().as_str()));
            }
        }
    }
    None
}

fn split_segment<'a>(current_repo_name: &Option<String>, segment: &'a str) -> Vec<&'a str> {
    lazy_static! {
        static ref EXTERNAL_REPO_REGEX: Regex = Regex::new(r#"@([A-Za-z0-9_-]+)"#).unwrap();
    }
    segment
        .lines()
        .filter(|e| {
            let mut line_ok = true;
            if let Some(repo) = &current_repo_name {
                for r in EXTERNAL_REPO_REGEX.captures_iter(e) {
                    if &r[1] != repo {
                        line_ok = false;
                    }
                }
            }
            line_ok
        })
        .collect()
}

pub async fn graph_query<B: BazelQuery, Q: AsRef<str>>(
    bazel_query: &B,
    query: Q,
) -> HashMap<String, HashSet<String>> {
    let res = bazel_query
        .execute(&vec![
            String::from("query"),
            String::from("--keep_going"),
            String::from("--output"),
            String::from("graph"),
            String::from(query.as_ref()),
        ])
        .await;

    let mut result: HashMap<String, HashSet<String>> = Default::default();

    let current_repo_name = parse_current_repo_name();

    let updated = res
        .stdout
        .lines()
        .skip(2)
        .map(|e| e.trim())
        .filter(|e| e.starts_with("\""));

    for ln in updated {
        eprintln!("Updated:\n{}", ln);

        let mut split_v = ln.split(" -> ");
        let lhs = split_v
            .next()
            .map(|e| e.replace("\"", ""))
            .map(|e| split_segment(&current_repo_name, &e));
        let rhs = split_v
            .next()
            .map(|e| e.replace("\"", ""))
            .map(|e| split_segment(&current_repo_name, &e));
        if let Some(lhs) = lhs {
            for &lhs in lhs.iter() {
                if let Some(rhs) = rhs {
                    for rhs in rhs.iter() {
                        if let Some(existing_rhs) = result.get_mut(rhs) {
                            existing_rhs.insert(lhs.to_string());
                        } else {
                            let mut hash_set = HashSet::default();
                            hash_set.insert(lhs.to_string());
                            result.insert(rhs.to_string(), hash_set);
                        }
                    }
                } else {
                    if let None = result.get(lhs) {
                        result.insert(lhs.to_string(), Default::default());
                    }
                }
            }
        } 
    }
    eprintln!("{:#?}", res);
    eprintln!("{:#?}", result);

    result
}
