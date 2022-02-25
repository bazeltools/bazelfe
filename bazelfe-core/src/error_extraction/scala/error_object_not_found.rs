use lazy_static::lazy_static;
use regex::Regex;

use super::ScalaClassImportRequest;

// Example usage:
// SCALA:
// package com.example
// import foo.bar.baz

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
    priority: i32,
) -> ScalaClassImportRequest {
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name,
        exact_only: false,
        src_fn: "extract_object_not_found",
        priority,
    }
}

pub(in crate::error_extraction::scala) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<ScalaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.scala):(\d+).*error: not found: object (.*)$").unwrap();
    }

    let mut result = None;
    let mut last_line_match = false;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();

                let class_name = captures.get(3).unwrap().as_str();

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
                                        1,
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
                        class_name.to_string(),
                        1,
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
        if last_line_match {}
        last_line_match = false
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_object_not_found_error() {
        let sample_output =
            "src/main/scala/com/example/Example.scala:40: error: not found: object foo
import foo.bar.baz
       ^
src/main/scala/com/example/Example.scala:40: warning: Unused import
import foo.bar.baz
               ^
one warning found
one error found
one warning found
one error found
java.lang.RuntimeException: Build failed
    at io.bazel.rulesscala.scalac.ScalacProcessor.compileScalaSources(ScalacProcessor.java:244)
    at io.bazel.rulesscala.scalac.ScalacProcessor.processRequest(ScalacProcessor.java:69)
    at io.bazel.rulesscala.worker.GenericWorker.runPersistentWorker(GenericWorker.java:45)
    at io.bazel.rulesscala.worker.GenericWorker.run(GenericWorker.java:111)
    at io.bazel.rulesscala.scalac.ScalaCInvoker.main(ScalaCInvoker.java:41)";
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![],
            },
        );
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/scala/com/example/Example.scala"),
                "foo".to_string(),
                1
            )])
        );
    }

    #[test]
    fn test_object_not_found_with_imports() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/scala/com/example/Example.scala"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 2,
                    prefix_section: String::from("foo.bar.baz"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/scala/com/example/Example.scala:2: error: not found: object foo
import foo.bar.baz
       ^
src/main/scala/com/example/Example.scala:2: warning: Unused import
import com.example.foo.bar.Baz
                           ^
one warning found
one error found";

        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/scala/com/example/Example.scala"),
                "foo.bar.baz".to_string(),
                50
            )])
        );
    }
}
