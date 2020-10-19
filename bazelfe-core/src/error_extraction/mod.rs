#[derive(Debug, PartialEq)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: String,
    pub priority: i32,
}

#[derive(Debug, PartialEq)]
pub struct ClassSuffixMatch {
    pub suffix: String,
    pub src_fn: String,
}

pub mod java;
pub mod scala;

pub fn extract_errors(target_kind: &Option<String>, input: &str) -> Vec<ClassImportRequest> {
    match target_kind.as_ref() {
        None => Vec::default(),
        Some(kind) => match kind.as_ref() {
            "scala_library" => scala::extract_errors(input),
            "scala_test" => scala::extract_errors(input),
            "java_library" => java::extract_errors(input),
            "java_test" => java::extract_errors(input),
            _ => Vec::default(),
        },
    }
}

pub fn extract_suffix_errors(target_kind: &Option<String>, input: &str) -> Vec<ClassSuffixMatch> {
    match target_kind.as_ref() {
        None => Vec::default(),
        Some(kind) => match kind.as_ref() {
            "scala_library" => scala::extract_suffix_errors(input),
            "scala_test" => scala::extract_suffix_errors(input),
            "java_library" => java::extract_suffix_errors(input),
            "java_test" => java::extract_suffix_errors(input),
            _ => Vec::default(),
        },
    }
}
