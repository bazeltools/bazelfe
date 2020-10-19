use std::collections::HashMap;

use nom::branch::alt;
use nom::multi::many0;
use nom::{
    bytes::complete::tag,
    combinator::{map},
    sequence::tuple,
    IResult,
};

fn consume_quoted_strings(ln: &str) -> IResult<&str, Vec<String>> {
    map(
        many0(tuple((
            many0(nom::character::complete::none_of("\"'")),
            alt((
                tuple((
                    tag("\""),
                    map(many0(nom::character::complete::none_of("\"")), |v| {
                        let s: String = v.into_iter().collect();
                        s
                    }),
                    tag("\""),
                )),
                tuple((
                    tag("'"),
                    map(many0(nom::character::complete::none_of("'")), |v| {
                        let s: String = v.into_iter().collect();
                        s
                    }),
                    tag("'"),
                )),
            )),
        ))),
        |vec| vec.into_iter().map(|e| (e.1).1).collect(),
    )(ln)
}

pub fn build_popularity_map() -> HashMap<String, usize> {
    let root_dir = std::env::current_dir().unwrap();

    let mut results: HashMap<String, usize> = HashMap::new();
    let mut to_visit = vec![root_dir.clone()];

    while !to_visit.is_empty() {
        let current_dir = to_visit.pop().unwrap();
        for entry in std::fs::read_dir(current_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let filename_str: String = path
                .file_name()
                .as_ref()
                .map(|e| e.clone())
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let file_type = std::fs::symlink_metadata(&path).unwrap();

            if file_type.is_dir() {
                to_visit.push(path);
            } else {
                let parent_path_str: String = path
                    .parent()
                    .unwrap()
                    .strip_prefix(&root_dir)
                    .unwrap()
                    .to_str()
                    .as_ref()
                    .map(|e| e.clone())
                    .unwrap()
                    .to_string();

                if file_type.is_file() && filename_str == "BUILD" {
                    let content = std::fs::read_to_string(path).unwrap();
                    for line in content.lines() {
                        for result in consume_quoted_strings(line).unwrap().1 {
                            let value = if result.starts_with(":") {
                                Some(format!("//{}{}", parent_path_str, result))
                            } else if result.starts_with("//") {
                                Some(result)
                            } else if result.starts_with("@") {
                                Some(result)
                            } else {
                                None
                            };
                            if let Some(r) = value {
                                let v = results.entry(r).or_insert(0);
                                *v += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    results
}
