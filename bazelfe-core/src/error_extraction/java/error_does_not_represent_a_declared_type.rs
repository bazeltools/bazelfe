use lazy_static::lazy_static;
use regex::Regex;

use super::JavaClassImportRequest;

// Example usage:
// This is the dagger annotation processor getting upset/confused alas.
// no good example to hand.

fn build_class_import_request(class_name: String) -> JavaClassImportRequest {
    JavaClassImportRequest {
        src_file_name: String::from("None"),
        class_name: class_name,
        exact_only: false,
        src_fn: "does_not_represent_a_declared_type",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<JavaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex =
            Regex::new(r"^\s*java.lang.RuntimeException: java.lang.IllegalArgumentException: ([A-Za-z0-9.<>_]+) does not represent a declared type.*$")
                .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let class_name = captures.get(1).unwrap().as_str();
                let class_import_request = build_class_import_request(class_name.to_string());
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
    fn test_sample_input() {
        let sample_output =
            "java.lang.RuntimeException: java.lang.IllegalArgumentException: com.example.foo.Bar does not represent a declared type
	at jdk.compiler/com.sun.tools.javac.api.JavacTaskImpl.handleExceptions(JavacTaskImpl.java:163)
	at jdk.compiler/com.sun.tools.javac.api.JavacTaskImpl.doCall(JavacTaskImpl.java:100)
	at jdk.compiler/com.sun.tools.javac.api.JavacTaskImpl.call(JavacTaskImpl.java:94)
	at com.google.devtools.build.buildjar.javac.BlazeJavacMain.compile(BlazeJavacMain.java:114)
	at com.google.devtools.build.buildjar.ReducedClasspathJavaLibraryBuilder.compileSources(ReducedClasspathJavaLibraryBuilder.java:57)";
        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                "com.example.foo.Bar".to_string()
            )])
        );
    }
}
