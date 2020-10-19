use std::{collections::HashMap, path::Path};

use crate::source_dependencies::ParsedFile;

mod error_cannot_access;
mod error_cannot_find_symbol;
mod error_indirect_dependency;
mod error_package_does_not_exist;

#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct JavaClassImportRequest {
    pub src_file_name: String,
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: &'static str,
    pub priority: i32,
}

impl JavaClassImportRequest {
    pub fn to_class_import_request(self) -> super::ClassImportRequest {
        super::ClassImportRequest {
            class_name: self.class_name,
            exact_only: self.exact_only,
            src_fn: format!("java::{}", self.src_fn),
            priority: self.priority,
        }
    }
}

fn do_load_file(path_str: &str) -> Option<ParsedFile> {
    let path = Path::new(path_str);

    if path.exists() {
        let file_contents = std::fs::read_to_string(path).unwrap();
        match crate::source_dependencies::java::parse_file(&file_contents) {
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
pub fn extract_errors(input: &str) -> Vec<super::ClassImportRequest> {
    let mut file_parse_cache: FileParseCache = FileParseCache::new();
    let combined_vec: Vec<super::ClassImportRequest> = vec![
        error_package_does_not_exist::extract(input, &mut file_parse_cache),
        error_indirect_dependency::extract(input),
        error_cannot_find_symbol::extract(input, &mut file_parse_cache),
    ]
    .into_iter()
    .flat_map(|e| e.into_iter().flat_map(|inner| inner.into_iter()))
    .map(|o| o.to_class_import_request())
    .collect();

    combined_vec
}

pub fn extract_suffix_errors(input: &str) -> Vec<super::ClassSuffixMatch> {
    vec![error_cannot_access::extract(input)]
        .into_iter()
        .flat_map(|e| e)
        .collect()
}
