use lazy_static::lazy_static;
use regex::Regex;

use super::ScalaClassImportRequest;

// Example usage:
// SCALA:
// package com.example
// class Foo extends asdf

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> ScalaClassImportRequest {
    let num_segments = class_name.chars().filter(|e| *e == '.').count();
    let priority: i32 = if num_segments < 2 {
        -5
    } else {
        num_segments as i32
    };
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name,
        exact_only: true,
        src_fn: "extract_value_or_type_not_found",
        priority,
    }
}

pub fn extract(input: &str) -> Vec<ScalaClassImportRequest> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.scala).*error: not found: (value|type) ([A-Za-z0-9._]+)$").unwrap();
    }

    let mut result: Vec<ScalaClassImportRequest> = Vec::default();
    let lines: Vec<&str> = input.lines().collect();
    for (ln_num, ln) in lines.iter().enumerate() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let class_name = captures.get(3).unwrap().as_str().to_string();

                let mut class_names = vec![];
                if ln_num < lines.len() - 1 {
                    for nme in lines[ln_num + 1]
                        .split_whitespace()
                        .flat_map(|e| {
                            let mut res: Vec<String> = Vec::default();
                            let mut buf = Vec::default();
                            for chr in e.chars() {
                                if chr.is_alphanumeric() || chr == '_' || chr == '.' {
                                    buf.push(chr);
                                } else if !buf.is_empty() {
                                    res.push(buf.into_iter().collect());
                                    buf = Vec::default();
                                }
                            }
                            if !buf.is_empty() {
                                res.push(buf.into_iter().collect());
                            }
                            res.into_iter()
                        })
                        .filter(|e| e.contains(&class_name))
                    {
                        class_names.push(nme);
                    }
                };
                class_names.push(class_name);

                let class_import_requests = class_names
                    .into_iter()
                    .map(|nme| build_class_import_request(src_file_name.to_string(), nme));

                result.extend(class_import_requests);
            }
        }
    }
    result.sort();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_value_not_found_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:8:: error: not found: value Foop
val myVal = Foop
            ^
one error found
one error found";

        assert_eq!(
            extract(sample_output),
            vec![build_class_import_request(
                String::from("src/main/scala/com/example/Example.scala"),
                "Foop".to_string()
            )]
        );
    }

    #[test]
    fn test_type_not_found_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:8: error: not found: type asdf
class Foo extends asdf
                    ^
one error found
one error found";

        assert_eq!(
            extract(sample_output),
            vec![build_class_import_request(
                String::from("src/main/scala/com/example/Example.scala"),
                "asdf".to_string()
            )]
        );
    }
}
