use std::path::PathBuf;

pub fn extract_classes_from_zip(path: PathBuf) -> Vec<String> {
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dump_missing_zip_contents() {
        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/zip_parse/non_existant_path.zip");

        let expected: Vec<String> = Vec::default();
        assert_eq!(extract_classes_from_zip(d), expected);
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
        let mut res = extract_classes_from_zip(d);
        expected.sort();
        res.sort();
        assert_eq!(res, expected);
    }
}
