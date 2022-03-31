use std::{collections::HashMap, path::Path};

use crate::source_dependencies::ParsedFile;

use super::ClassSuffixMatch;

mod error_is_not_a_member_of_generic;
mod error_is_not_a_member_of_package;
mod error_object_not_found;
mod error_symbol_is_missing_from_classpath;
mod error_symbol_type_missing_from_classpath;
mod error_value_not_found;

#[derive(Debug, PartialEq, Clone, PartialOrd, Ord, Eq)]
pub struct ScalaClassImportRequest {
    pub src_file_name: String,
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: i32,
}

impl ScalaClassImportRequest {
    pub fn to_class_import_request(self) -> super::ClassImportRequest {
        super::ClassImportRequest {
            class_name: self.class_name,
            exact_only: self.exact_only,
            src_fn: format!("scala::{}", self.src_fn),
            priority: self.priority,
        }
    }
}

fn do_load_file(path_str: &str) -> Option<ParsedFile> {
    let path = Path::new(path_str);

    if path.exists() {
        let file_contents = std::fs::read_to_string(path).unwrap();
        match crate::source_dependencies::scala::parse_file(&file_contents) {
            Err(_) => None,
            Ok(file) => Some(file),
        }
    } else {
        None
    }
}

pub(in crate::error_extraction) struct FileParseCache {
    file_parse_cache: HashMap<String, ParsedFile>,
}
impl FileParseCache {
    pub fn new() -> Self {
        Self {
            file_parse_cache: HashMap::new(),
        }
    }
    // used in tests
    #[allow(dead_code)]
    pub fn init_from_par(key: String, v: ParsedFile) -> Self {
        let mut map = HashMap::new();
        map.insert(key, v);
        Self {
            file_parse_cache: map,
        }
    }
    pub fn load_file(&mut self, file_path: &str) -> Option<&ParsedFile> {
        if !self.file_parse_cache.contains_key(file_path) {
            if let Some(parsed_file) = do_load_file(file_path) {
                self.file_parse_cache
                    .insert(file_path.to_string(), parsed_file);
            }
        }
        self.file_parse_cache.get(file_path)
    }
}

pub fn extract_errors(input: &str) -> Vec<super::ActionRequest> {
    let mut file_parse_cache: FileParseCache = FileParseCache::new();
    let combined_vec: Vec<super::ActionRequest> = vec![
        error_is_not_a_member_of_package::extract(input, &mut file_parse_cache),
        error_is_not_a_member_of_generic::extract(input, &mut file_parse_cache),
        error_object_not_found::extract(input, &mut file_parse_cache),
        error_symbol_is_missing_from_classpath::extract(input),
        error_symbol_type_missing_from_classpath::extract(input),
        Some(error_value_not_found::extract(input)),
    ]
    .into_iter()
    .flat_map(|e| e.into_iter().flat_map(|inner| inner.into_iter()))
    .flat_map(|e| {
        let cached_file_data = file_parse_cache.load_file(&e.src_file_name);

        match cached_file_data {
            None => vec![e],
            Some(file_data) => {
                let extra_wildcard_imports: Vec<ScalaClassImportRequest> = file_data
                    .imports
                    .iter()
                    .filter_map(|e| match e.suffix {
                        crate::source_dependencies::SelectorType::SelectorList(_) => None,
                        crate::source_dependencies::SelectorType::WildcardSelector => {
                            Some(&e.prefix_section)
                        }
                        crate::source_dependencies::SelectorType::NoSelector => None,
                    })
                    .chain(file_data.package_name.iter())
                    .flat_map(|prefix| {
                        let elements = e.class_name.chars().filter(|e| *e == '.').count();
                        if elements < 3 {
                            Some(ScalaClassImportRequest {
                                class_name: format!("{}.{}", prefix, e.class_name),
                                priority: -3,
                                exact_only: true,
                                ..e.clone()
                            })
                        } else {
                            None
                        }
                    })
                    .collect();

                extra_wildcard_imports
                    .into_iter()
                    .chain(vec![e].into_iter())
                    .collect()
            }
        }
        .into_iter()
        .map(|o| {
            if o.class_name.find('.').is_none() {
                let suffix = ClassSuffixMatch {
                    suffix: o.class_name,
                    src_fn: o.src_fn.to_string(),
                    priority: 0,
                };
                debug!("Found class suffix request: {:#?}", suffix);
                super::ActionRequest::Suffix(suffix)
            } else if let Some(prefix) = o.class_name.strip_prefix("<root>.") {
                let prefix_match = crate::error_extraction::ClassImportRequest {
                    class_name: prefix.to_string(),
                    exact_only: false,
                    src_fn: o.src_fn.to_string(),
                    priority: o.priority,
                };

                debug!("Found class root prefix request: {:#?}", prefix_match);
                super::ActionRequest::Prefix(prefix_match)
            } else if let Some(suffix) = o
                .class_name
                .strip_prefix("<none>.")
                .or_else(|| o.class_name.strip_prefix("<none>"))
                .or_else(|| o.class_name.strip_prefix("<root>."))
            {
                let suffix_match = ClassSuffixMatch {
                    suffix: suffix.to_string(),
                    src_fn: o.src_fn.to_string(),
                    priority: 5,
                };
                debug!("Found class suffix request: {:#?}", suffix_match);
                super::ActionRequest::Suffix(suffix_match)
            } else {
                let r = o.to_class_import_request();
                debug!("Found class import request: {:#?}", r);
                super::ActionRequest::Prefix(r)
            }
        })
    })
    .filter(|e| match e {
        super::ActionRequest::Prefix(_) => true,
        super::ActionRequest::Suffix(s) => s.suffix.chars().filter(|e| *e == '.').count() > 0,
    })
    .collect();

    combined_vec
}
