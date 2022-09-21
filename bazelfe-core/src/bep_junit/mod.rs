mod failed_action;
pub mod junit_xml_error_writer;
mod test_results_ops;
mod xml_utils;

pub use failed_action::emit_junit_xml_from_aborted_action;
pub use failed_action::emit_junit_xml_from_failed_action;
pub use test_results_ops::{emit_backup_error_data, suites_with_error_from_xml};

// This is to just take the label and provide a sane output path
// in the resulting junit root to avoid conflicts.
pub fn label_to_junit_relative_path(label: &str) -> String {
    let p: String = if let Some(external_suffix) = label.strip_prefix('@') {
        format!("external/{}", external_suffix)
    } else if let Some(internal_suffix) = label.strip_prefix("//") {
        internal_suffix.to_string()
    } else {
        label.to_string()
    };

    p.replace("//", "/").replace(':', "/")
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_internal_label_to_expected_path() {
        assert_eq!(
            label_to_junit_relative_path("//src/main/foo/bar/baz:lump"),
            "src/main/foo/bar/baz/lump".to_string()
        );
    }

    #[test]
    fn test_external_label_to_expected_path() {
        assert_eq!(
            label_to_junit_relative_path("@my_lib//src/main/foo/bar/baz:lump"),
            "external/my_lib/src/main/foo/bar/baz/lump".to_string()
        );
    }
}
