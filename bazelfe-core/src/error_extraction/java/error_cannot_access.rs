use lazy_static::lazy_static;
use regex::Regex;

use super::super::ClassSuffixMatch;
static SRC_FN: &str = "java::error_cannot_access";

pub fn extract(input: &str) -> Vec<ClassSuffixMatch> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^(.*\.java).* error: cannot access ([A-Za-z0-9.<>_]+)\s*$").unwrap();
    }

    let mut result = vec![];
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let suffix = captures.get(2).unwrap().as_str();
                let class_import_request = ClassSuffixMatch {
                    suffix: String::from(suffix),
                    src_fn: String::from(SRC_FN),
                };
                result.push(class_import_request);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    fn test_not_a_member_of_package_error() {
        let sample_output =
            "src/main/java/com/example/foo/bar/Baz.java:205: error: cannot access JSONObject
    Blah key = Blah.myfun(jwk);
";
        assert_eq!(
            extract(sample_output),
            vec![ClassSuffixMatch {
                suffix: String::from("JSONObject"),
                src_fn: String::from(SRC_FN)
            }]
        );
    }
}
