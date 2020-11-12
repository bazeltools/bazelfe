use lazy_static::lazy_static;
use regex::Regex;

use crate::source_dependencies::ParsedFile;

use super::JavaClassImportRequest;

// Example usage:
// JAVA:
// package com.example;
// import com.example.foo.bar.Baz;

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> JavaClassImportRequest {
    JavaClassImportRequest {
        src_file_name: source_file_name,
        class_name: class_name,
        exact_only: false,
        src_fn: "cannot_find_symbol",
        priority: 1,
    }
}

fn build_class_import_request_low_priority(
    source_file_name: String,
    class_name: String,
) -> JavaClassImportRequest {
    JavaClassImportRequest {
        src_file_name: source_file_name,
        class_name: class_name,
        exact_only: true,
        src_fn: "cannot_find_symbol",
        priority: -50,
    }
}

fn extract_symbol_with_package(
    lines: &Vec<String>,
    src_file_name: &String,
    result: &mut Vec<JavaClassImportRequest>,
) {
    lazy_static! {
        static ref SYMBOL_RE: Regex = Regex::new(r"^\s*symbol:\s*class\s*(.*)$").unwrap();
        static ref PACKAGE_RE: Regex = Regex::new(r"^\s*location:\s*package\s*(.*)$").unwrap();
    }

    let symbol_capture = SYMBOL_RE.captures(&lines[2]);
    let package_capture = PACKAGE_RE.captures(&lines[3]);

    match (symbol_capture, package_capture) {
        (Some(c1), Some(c2)) => {
            let class_name = format!(
                "{}.{}",
                c2.get(1).unwrap().as_str(),
                c1.get(1).unwrap().as_str()
            );
            let class_import_request =
                build_class_import_request(src_file_name.to_string(), class_name);

            result.push(class_import_request);
        }
        _ => (),
    }
}

fn extract_symbol(
    lines: &Vec<String>,
    src_file_name: &String,
    parsed_file: &ParsedFile,
    result: &mut Vec<JavaClassImportRequest>,
) {
    lazy_static! {
        // We match a slew of things like locations around methods, or otherwise, we don't mind 
        // where these come from. We will attempt to use the symbol info to figure out the lookup
        static ref SYMBOL_RE: Regex =
            Regex::new(r"^\s*symbol:\s*(class|variable)\s*(.*)$").unwrap();
    }

    let symbol_capture = SYMBOL_RE.captures(&lines[2]);

    if let Some(ref capture) = symbol_capture {
        let missing_symbol = capture.get(2).unwrap().as_str();
        let mut packages: Vec<String> = Vec::new();
        if let Some(ref package_name) = &parsed_file.package_name {
            packages.push(package_name.clone());
        }

        for import in parsed_file.imports.iter() {
            match import.suffix {
                crate::source_dependencies::SelectorType::SelectorList(_) => (),
                crate::source_dependencies::SelectorType::NoSelector => (),
                crate::source_dependencies::SelectorType::WildcardSelector => {
                    packages.push(import.prefix_section.clone())
                }
            }
        }

        // this is a high priority one, if it matches we will ignore the others/clear them out.
        for import in parsed_file.imports.iter() {
            match import.suffix {
                crate::source_dependencies::SelectorType::SelectorList(_) => (),
                crate::source_dependencies::SelectorType::NoSelector => {
                    if import.prefix_section.ends_with(missing_symbol) {
                        // If we find matching imports, these are ~pretty high quality signals of what we are looking for is this.
                        // Zero out the lower quality signals and just inject these with higher priority
                        packages.clear();
                        result.push(JavaClassImportRequest {
                            src_file_name: src_file_name.to_string(),
                            class_name: import.prefix_section.clone(),
                            exact_only: false,
                            src_fn: "cannot_find_symbol_matched_import",
                            priority: 6,
                        })
                    }
                }
                crate::source_dependencies::SelectorType::WildcardSelector => (),
            }
        }

        for package_name in packages {
            let class_name = format!("{}.{}", package_name, missing_symbol);
            let class_import_request =
                build_class_import_request_low_priority(src_file_name.to_string(), class_name);

            result.push(class_import_request);
        }
    }
}

pub(in crate::error_extraction) fn extract(
    input: &str,
    file_parse_cache: &mut super::FileParseCache,
) -> Option<Vec<JavaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.java):(\d+):.*error: cannot find symbol\s*$").unwrap();
    }

    let mut result = Vec::default();
    let mut batch_result = Vec::default();
    let mut process_batch: Option<(Vec<String>, String)> = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);
        if let Some((ref mut vec, ref src_file_name)) = process_batch {
            if vec.len() < 3 {
                vec.push(ln.to_string());
                // on ~just len == 3
                // the other branch of the outer if may never fire if there is only 3 lines.
                if vec.len() == 3 {
                    // we only need 3 lines of an error to try extract the symbol information
                    if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                        extract_symbol(vec, src_file_name, file_data, &mut batch_result);
                    }
                }
            } else {
                vec.push(ln.to_string());

                let mut tmp = Vec::default();
                // If given the use of the 4th line we extracted with a package too, this is higher priority
                // so ditch the symbol info and use this instead!
                extract_symbol_with_package(vec, src_file_name, &mut tmp);

                if tmp.is_empty() {
                    result.extend(batch_result.drain(..));
                } else {
                    result.extend(tmp.drain(..));
                }
                batch_result.clear();

                process_batch = None;
            }
        }
        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();

                process_batch = Some((Vec::new(), src_file_name.to_string()));
                let src_line_number: u32 = captures.get(2).unwrap().as_str().parse().unwrap();

                if let Some(file_data) = file_parse_cache.load_file(src_file_name) {
                    for e in file_data.imports.iter() {
                        if e.line_number == src_line_number {
                            let class_import_request = build_class_import_request(
                                src_file_name.to_string(),
                                e.prefix_section.to_string(),
                            );
                            result.push(class_import_request);
                            // Don't bother using fallback mechanisms, we have an import matched.
                            process_batch = None;
                        }
                    }
                }
            }
        }
    }
    result.extend(batch_result.drain(..));
    result.sort();
    result.dedup();
    Some(result)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_not_a_member_of_package_on_import_line() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    // this should get ignorred since we are on a package item.
                    prefix_section: String::from("javax.foo.bar.baz.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:16: error: cannot find symbol
    import javax.annotation.Nullable;
                           ^
      symbol:   class Nullable
      location: package javax.annotation";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/foo/Example.java"),
                "javax.foo.bar.baz.annotation.Nullable".to_string()
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    // this should get ignorred since we are on a package item.
                    prefix_section: String::from("javax.foo.bar.baz.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:22: error: cannot find symbol
    import javax.annotation.Nullable;
                           ^
      symbol:   class Nullable
      location: package javax.annotation";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/foo/Example.java"),
                "javax.annotation.Nullable".to_string()
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_non_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: None,
                imports: vec![],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:16: error: cannot find symbol
    import javax.annotation.Nullable;
                           ^
      symbol:   class Nullable
      location: package javax.annotation";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request(
                String::from("src/main/java/com/example/foo/Example.java"),
                "javax.annotation.Nullable".to_string()
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_class_symbol() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: Some(String::from("com.example.foo")),
                imports: vec![],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:16: error: cannot find symbol
        FooBarBaz.class,
        ^
  symbol:   class FooBarBaz
  location: class UsingClass";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![build_class_import_request_low_priority(
                String::from("src/main/java/com/example/foo/Example.java"),
                "com.example.foo.FooBarBaz".to_string()
            )])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_class_symbol_with_wildcard_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: Some(String::from("com.example.foo")),
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    prefix_section: String::from("javax.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::WildcardSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:19: error: cannot find symbol
        FooBarBaz.class,
        ^
  symbol:   variable FooBarBaz
  location: class UsingClass";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![
                build_class_import_request_low_priority(
                    String::from("src/main/java/com/example/foo/Example.java"),
                    "com.example.foo.FooBarBaz".to_string()
                ),
                build_class_import_request_low_priority(
                    String::from("src/main/java/com/example/foo/Example.java"),
                    "javax.annotation.Nullable.FooBarBaz".to_string()
                )
            ])
        );
    }

    #[test]
    fn test_not_a_member_of_package_error_class_symbol_with_matching_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: Some(String::from("com.example.foo")),
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    prefix_section: String::from("javax.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:19: error: cannot find symbol
            Nullable.class,
        ^
  symbol:   variable Nullable
  location: class UsingClass";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![JavaClassImportRequest {
                src_file_name: String::from("src/main/java/com/example/foo/Example.java"),
                class_name: String::from("javax.annotation.Nullable"),
                exact_only: false,
                src_fn: "cannot_find_symbol_matched_import",
                priority: 6,
            }])
        );
    }

    #[test]
    fn test_not_a_member_of_package_symbol_with_matching_import() {
        let mut file_cache = super::super::FileParseCache::init_from_par(
            String::from("src/main/java/com/example/foo/Example.java"),
            crate::source_dependencies::ParsedFile {
                package_name: Some(String::from("com.example.foo")),
                imports: vec![crate::source_dependencies::Import {
                    line_number: 16,
                    prefix_section: String::from("javax.annotation.Nullable"),
                    suffix: crate::source_dependencies::SelectorType::NoSelector,
                }],
            },
        );
        let sample_output =
            "src/main/java/com/example/foo/Example.java:19: error: cannot find symbol
            Nullable.class,
        ^
  symbol:   class Nullable";
        assert_eq!(
            extract(sample_output, &mut file_cache),
            Some(vec![JavaClassImportRequest {
                src_file_name: String::from("src/main/java/com/example/foo/Example.java"),
                class_name: String::from("javax.annotation.Nullable"),
                exact_only: false,
                src_fn: "cannot_find_symbol_matched_import",
                priority: 6,
            }])
        );
    }

}
