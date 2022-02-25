#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClassImportRequest {
    pub class_name: String,
    pub exact_only: bool,
    pub src_fn: String,
    pub priority: i32,
}
impl PartialOrd for ClassImportRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ClassImportRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.class_name.cmp(&other.class_name))
            .then_with(|| self.src_fn.cmp(&other.src_fn))
            .then_with(|| self.exact_only.cmp(&other.exact_only))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassSuffixMatch {
    pub suffix: String,
    pub src_fn: String,
    pub priority: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionRequest {
    Prefix(ClassImportRequest),
    Suffix(ClassSuffixMatch),
}
impl ActionRequest {
    pub fn priority(&self) -> i32 {
        match self {
            ActionRequest::Prefix(p) => p.priority,
            ActionRequest::Suffix(s) => s.priority,
        }
    }
}

impl Ord for ActionRequest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.priority().cmp(&other.priority()).reverse()
    }
}
impl PartialOrd for ActionRequest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub mod java;
pub mod scala;

pub fn extract_errors(target_kind: &Option<String>, input: &str) -> Vec<ActionRequest> {
    debug!("Extract errors seeeing target kind: {:#?}", target_kind);
    let matched = target_kind.as_ref().and_then(|kind| match kind.as_ref() {
        "scala_library" => {
            let mut errors = scala::extract_errors(input);
            errors.extend(java::extract_errors(input));
            Some(errors)
        }
        "scala_test" => Some(scala::extract_errors(input)),
        "java_library" => Some(java::extract_errors(input)),
        "java_test" => Some(java::extract_errors(input)),
        _ => None,
    });

    if let Some(existing) = matched {
        existing
    } else {
        let mut v = scala::extract_errors(input);
        v.extend(java::extract_errors(input).into_iter());
        v
    }
}
