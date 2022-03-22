use lazy_static::lazy_static;
use regex::Regex;

use super::JavaClassImportRequest;

// Example usage:
// JAVA:
// package com.example;
// import com.example.foo.bar.Baz;

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
    priority: i32,
) -> JavaClassImportRequest {
    JavaClassImportRequest {
        src_file_name: source_file_name,
        class_name,
        exact_only: false,
        src_fn: "package_does_not_exist",
        priority,
    }
}

pub(in crate::error_extraction) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<JavaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^(.*\.java):(\d+):.*error: package ([A-Za-z0-9.<>_]+).* does not exist\s*$"
        )
        .unwrap();
    }

    let mut result = None;
    let lines: Vec<&str> = input.lines().collect();
    for (pos, ln) in lines.iter().enumerate() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();
                let package = captures.get(3).unwrap().as_str();

                let mut class_import_request = Vec::default();
                if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                    for e in file_data.imports.iter() {
                        if e.line_number == src_line_number {
                            class_import_request.push(build_class_import_request(
                                src_file_name.to_string(),
                                e.prefix_section.to_string(),
                                30,
                            ));
                        }
                    }
                } else if pos < lines.len() - 1 {
                    let target_line = lines[pos + 1];
                    match crate::source_dependencies::java::parse_imports(target_line) {
                        Ok(matched) => {
                            if let Some(e) = matched.into_iter().next() {
                                class_import_request.push(build_class_import_request(
                                    src_file_name.to_string(),
                                    e.prefix_section.to_string(),
                                    30,
                                ));
                            }
                        }
                        Err(_) => (),
                    }
                }

                if class_import_request.is_empty() {
                    if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                        if let Some(pkg) = file_data.package_name.as_ref() {
                            class_import_request.push(JavaClassImportRequest {
                                src_file_name: src_file_name.to_string(),
                                class_name: format!("{}.{}", pkg, package.to_string()),
                                exact_only: true,
                                src_fn: "package_does_not_exist",
                                priority: 2,
                            });
                        }
                    }

                    class_import_request.push(build_class_import_request(
                        src_file_name.to_string(),
                        package.to_string(),
                        2,
                    ));
                }
                result = match result {
                    None => Some(class_import_request),
                    Some(ref mut inner) => {
                        inner.append(&mut class_import_request);
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
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![],
            },
        );

        let sample_output =
            "src/main/java/com/example/Example.java:3: error: package com.google.common.base does not exist
    import com.google.common.base.Preconditions;
";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/Example.java"),
                "com.google.common.base".to_string(),
                2
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_with_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 3,
                    prefix_section: String::from("com.google.common.base.Preconditions"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );

        let sample_output =
                "src/main/java/com/example/Example.java:3: error: package com.google.common.base does not exist
        import com.google.common.base.Preconditions;
    ";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/Example.java"),
                "com.google.common.base.Preconditions".to_string(),
                30
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_inner_class() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: Some(String::from("com.example")),
                imports: vec![crate::source_dependencies::Import {
                    line_number: 3,
                    prefix_section: String::from("com.google.common.base.Preconditions"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );

        // non existant path.
        let sample_output =
            "src/main/java/com/example/Example.java:33: error: package FooBarBaz does not exist
        FooBarBaz.;
    ";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![
                JavaClassImportRequest {
                    src_file_name: String::from("src/main/java/com/example/Example.java"),
                    class_name: String::from("com.example.FooBarBaz"),
                    exact_only: true,
                    src_fn: "package_does_not_exist",
                    priority: 2,
                },
                build_class_import_request(
                    String::from("src/main/java/com/example/Example.java"),
                    "FooBarBaz".to_string(),
                    2
                )
            ])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_with_no_file_parse_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 3,
                    prefix_section: String::from("com.google.common.base.Preconditions"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );

        // non existant path.
        let sample_output =
                "foo/bar/baz/doh/src/main/java/com/example/Example.java:3: error: package com.google.common.base does not exist
        import com.google.common.base.Preconditions;
    ";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("foo/bar/baz/doh/src/main/java/com/example/Example.java"),
                "com.google.common.base.Preconditions".to_string(),
                30
            )])
        );
    }
}
