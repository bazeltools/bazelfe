mod failed_action;
pub mod junit_xml_error_writer;

pub use failed_action::emit_junit_xml_from_failed_action;

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

    p.replace("//", "/").replace(":", "/")
}
