use std::path::PathBuf;

fn extract_paths_from_zip(path: PathBuf) -> Vec<String> {
    if !path.exists() {
        return Vec::default();
    }
    let mut results = Vec::default();

    let file = std::fs::File::open(&path).unwrap();

    let archive = zip::ZipArchive::new(file).unwrap();

    for i in archive.file_names() {
        results.push(i.to_string());
    }

    results
}

use lazy_static::lazy_static;
use regex::Regex;

fn remove_from<'a>(haystack: &'a str, needle: &str) -> &'a str {
    match haystack.find(needle) {
        None => haystack,
        Some(pos) => &haystack[0..pos],
    }
}
fn transform_file_names_into_class_names(class_names: Vec<String>) -> Vec<String> {
    lazy_static! {
        static ref SUFFIX_ANON_CLAZZES: Regex = Regex::new(r"(\$\d*)?\.class$").unwrap();
    }

    let mut vec: Vec<String> = class_names
        .into_iter()
        .filter_map(|e| {
            if !e.starts_with("META-INF")
                && e.ends_with(".class")
                && !(e.contains("/$") && e.contains("$/"))
            {
                Some(remove_from(&SUFFIX_ANON_CLAZZES.replace(&e, ""), "$$").to_string())
            } else {
                None
            }
        })
        .map(|e| e.replace("/$", "/").replace(['$', '/'], "."))
        .collect();
    vec.sort();
    vec.dedup();
    vec
}

pub fn extract_classes_from_zip(path: PathBuf) -> Vec<String> {
    transform_file_names_into_class_names(extract_paths_from_zip(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_missing_zip_contents() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/zip_parse/non_existant_path.zip");

        let expected: Vec<String> = Vec::default();
        assert_eq!(extract_paths_from_zip(d), expected);
    }

    #[test]
    fn dump_zip_contents() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/zip_parse/sample.zip");

        let mut expected: Vec<String> = vec![
            String::from("a.txt"),
            String::from("b.jar"),
            String::from("e.foo"),
        ];
        let mut res = extract_paths_from_zip(d);
        expected.sort();
        res.sort();
        assert_eq!(res, expected);
    }

    #[test]
    fn sanitize_file_name_to_class_name() {
        let sample_inputs = vec![
            "scala/reflect/internal/SymbolPairs$Cursor$$anon$1.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anon$2.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anonfun$init$2$$anonfun$apply$1.class",
            "scala/reflect/internal/SymbolPairs$Cursor$$anonfun$init$2$$anonfun$apply$2.class",
            "scala/reflect/internal/ReificationSupport$ReificationSupportImpl$UnMkTemplate$$anonfun$ctorArgsCorrespondToFields$1$1.class",
            "scala/reflect/internal/Depth$.class",
            "scala/reflect/internal/Depth.class",
            "com/android/aapt/Resources$AllowNew$1.class",
            "com/android/aapt/Resources$AllowNew$Builder.class",
            "com/android/aapt/Resources$AllowNew.class",
            "com/android/aapt/Resources$AllowNewOrBuilder.class",
            "com/android/aapt/$shaded$/Resources$AllowNewOrBuilder.class",
            "com/android/aapt/$Foo.class",
        ];

        let expected_results: Vec<String> = vec![
            "com.android.aapt.Foo",
            "com.android.aapt.Resources.AllowNew",
            "com.android.aapt.Resources.AllowNew.Builder",
            "com.android.aapt.Resources.AllowNewOrBuilder",
            "scala.reflect.internal.Depth",
            "scala.reflect.internal.ReificationSupport.ReificationSupportImpl.UnMkTemplate",
            "scala.reflect.internal.SymbolPairs.Cursor",
        ]
        .into_iter()
        .map(|e| e.to_string())
        .collect();

        assert_eq!(
            transform_file_names_into_class_names(
                sample_inputs.into_iter().map(|e| e.to_string()).collect()
            ),
            expected_results
        );
    }
}
