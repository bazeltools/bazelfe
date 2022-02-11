use std::collections::HashMap;

use nom::branch::alt;
use nom::multi::many0;
use nom::{bytes::complete::tag, combinator::map, sequence::tuple, IResult};

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
use walkdir::{DirEntry, WalkDir};

use crate::label_utils::sanitize_label;

fn is_not_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() == 0 || !s.starts_with('.'))
        .unwrap_or(false)
}

pub async fn build_popularity_map() -> HashMap<String, usize> {
    let mut results: HashMap<String, usize> = HashMap::new();
    let mut join_handles = Vec::default();

    WalkDir::new(".")
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| (e.file_type().is_dir() && is_not_hidden(e)) || e.file_name() == "BUILD")
        .filter_map(|v| v.ok())
        .filter(|e| e.file_type().is_file())
        .for_each(|x| {
            // println!("{:#?}", x);
            join_handles.push(tokio::task::spawn(async move {
                let path = x.path();
                let mut local_result: Vec<String> = Vec::default();
                let content = tokio::fs::read_to_string(path).await.unwrap();
                for line in content.lines() {
                    for result in consume_quoted_strings(line).unwrap().1 {
                        let value = if result.starts_with(':') {
                            Some(format!(
                                "//{}{}",
                                path.parent()
                                    .unwrap()
                                    .to_str()
                                    .unwrap()
                                    .strip_prefix("./")
                                    .unwrap(),
                                result
                            ))
                        } else if result.starts_with("//") || result.starts_with('@') {
                            Some(result)
                        } else {
                            None
                        };
                        if let Some(r) = value {
                            local_result.push(sanitize_label(r));
                        }
                    }
                }
                local_result
            }));
        });

    for join_handle in join_handles {
        for r in join_handle.await.unwrap().into_iter() {
            let v = results.entry(r).or_insert(0);
            *v += 1;
        }
    }

    results
}
