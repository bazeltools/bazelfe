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
    priority: i32,
) -> ScalaClassImportRequest {
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name,
        exact_only: true,
        src_fn: "extract_not_a_member_of_generic",
        priority,
    }
}

pub(in crate::error_extraction::scala) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<ScalaClassImportRequest>> {
    lazy_static! {
        static ref MEMBER_OF_CLASS_OR_OBJECT: Regex = Regex::new(
            r"^(.*\.scala):(\d+):.*error: \w* (\w*) is not a member of ([A-Za-z0-9.<>_]+)\s*$"
        )
        .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = MEMBER_OF_CLASS_OR_OBJECT.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();

                let class_or_package_component = captures.get(3).unwrap().as_str();
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
                                            format!("{}.{}", e.prefix_section, orig),
                                            10,
                                        ));
                                    }
                                }
                                crate::source_dependencies::SelectorType::WildcardSelector => v
                                    .push(build_class_import_request(
                                        src_file_name.to_string(),
                                        e.prefix_section.to_string(),
                                        9,
                                    )),
                                crate::source_dependencies::SelectorType::NoSelector => {
                                    v.push(build_class_import_request(
                                        src_file_name.to_string(),
                                        e.prefix_section.to_string(),
                                        50,
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
                        format!("{}.{}", package, class_or_package_component),
                        5,
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
    fn test_not_a_member_of_class() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![],
            },
        );
        let sample_output =
            "src/test/scala/com/foo/bar/Baz.scala:21: error: value myFunction is not a member of com.example.foo.ObjectName
possible cause: maybe a semicolon is missing before `value myFunction'?
      .myFunction(data)
       ^
";

        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/test/scala/com/foo/bar/Baz.scala"),
                "com.example.foo.ObjectName.myFunction".to_string(),
                5
            )])
        );
    }
}
