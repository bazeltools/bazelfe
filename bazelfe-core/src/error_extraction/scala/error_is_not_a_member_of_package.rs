use lazy_static::lazy_static;
use regex::Regex;

use super::ScalaClassImportRequest;

// Example usage:
// SCALA:
// package com.example
// import com.example.foo.bar.Baz

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> ScalaClassImportRequest {
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name: class_name,
        exact_only: false,
        src_fn: "extract_not_a_member_of_package",
        priority: 1,
    }
}

pub(in crate::error_extraction::scala) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<ScalaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^(.*\.scala):(\d+):.*error: \w* (\w*) is not a member of package ([A-Za-z0-9.<>_]+).*$"
        )
        .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();

                let class_name = captures.get(3).unwrap().as_str();
                let package = captures.get(4).unwrap().as_str();

                let mut class_import_request = None;
                if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                    for e in file_data.imports.iter() {
                        if e.line_number == src_line_number {
                            let mut v = Vec::default();
                            match &e.suffix {
                                crate::source_dependencies::SelectorType::SelectorList(lst) => {
                                    for (orig, _) in lst {
                                        v.push(build_class_import_request(
                                            src_file_name.to_string(),
                                            format!("{}.{}", e.prefix_section.to_string(), orig),
                                        ));
                                    }
                                }
                                crate::source_dependencies::SelectorType::WildcardSelector => v
                                    .push(build_class_import_request(
                                        src_file_name.to_string(),
                                        e.prefix_section.to_string(),
                                    )),
                                crate::source_dependencies::SelectorType::NoSelector => {
                                    v.push(build_class_import_request(
                                        src_file_name.to_string(),
                                        e.prefix_section.to_string(),
                                    ))
                                }
                            };
                            class_import_request = Some(v);
                        }
                    }
                }

                if class_import_request.is_none() {
                    class_import_request = Some(vec![build_class_import_request(
                        src_file_name.to_string(),
                        format!("{}.{}", package, class_name),
                    )]);
                }

                result = match result {
                    None => Some(class_import_request.unwrap()),
                    Some(ref mut inner) => {
                        inner.extend(class_import_request.unwrap().into_iter());
                        result
                    }
                };
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_not_a_member_of_package_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:2: error: object foo is not a member of package com.example
import com.example.foo.bar.Baz
                   ^
src/main/scala/com/example/Example.scala:2: warning: Unused import
import com.example.foo.bar.Baz
                           ^
one warning found
one error found";

        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                String::from("src/main/scala/com/example/Example.scala"),
                "com.example.foo".to_string()
            )])
        );
    }
}
