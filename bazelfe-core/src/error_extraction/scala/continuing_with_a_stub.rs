use lazy_static::lazy_static;
use regex::Regex;

use super::ScalaClassImportRequest;

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> ScalaClassImportRequest {
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name,
        exact_only: false,
        src_fn: "extract_continuing_with_a_stub",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<ScalaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^\s*(.*\.scala).*error: Class\s*([A-Za-z0-9.<>_]+)\s*not found - continuing with a stub.$"
        )
        .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln).map(|captures| {
            (
                captures.get(1).unwrap().as_str(),
                captures.get(2).unwrap().as_str(),
            )
        });

        match captures {
            None => (),
            Some((src_file_name, class_name)) => {
                let class_import_request =
                    build_class_import_request(src_file_name.to_string(), class_name.to_string());
                result = match result {
                    None => Some(vec![class_import_request]),
                    Some(ref mut inner) => {
                        inner.push(class_import_request);
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
    fn test_continue_with_stub() {
        let sample_output ="
        src/main/scala/com/example/D.scala:50: error: Class com.example.a.b.c.d.E not found - continuing with a stub.
      OtherThingy.SomeConstant,
      ^
one error found
Build failed";

        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                String::from("src/main/scala/com/example/D.scala"),
                "com.example.a.b.c.d.E".to_string()
            )])
        );
    }
}
