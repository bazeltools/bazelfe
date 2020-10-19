use lazy_static::lazy_static;
use regex::Regex;

use super::ScalaClassImportRequest;

// Example usage:
// This is with one plus deps enabled in rules scala
// each target depends on the target before it in the alphabet directly.
// SCALA:
//
// A.scala / target name "A"
//package com.example.a
// object Aobject {
//     val value = "asdf"
//   }
//   trait ATrait {
//     def fooA: String
//   }
//
// B.scala / target name "B"
// package com.example.b
// object Bobject {
//   val value = "asdf"
// }
// trait BTrait {
//   def fooB: String
// }
// trait BTraitExtendsA extends com.example.a.ATrait {
//   def fooB: String
// }
//
// C.scala / target name "C"
// package com.example.c
// object Cobject {
//   val value = "asdf"
// }
// trait CTraitExtendsBTraitExtendsATrait extends com.example.b.BTraitExtendsA {
//   def fooC: String
// }
//
// D.scala / target name "D"
// package com.example.d
// object Dobject {
//   val value = "asdf"
// }
// object DClass extends com.example.c.CTraitExtendsBTraitExtendsATrait

fn build_class_import_request(
    source_file_name: String,
    class_name: String,
) -> ScalaClassImportRequest {
    ScalaClassImportRequest {
        src_file_name: source_file_name,
        class_name: class_name,
        exact_only: false,
        src_fn: "extract_symbol_is_missing_from_classpath",
        priority: 1,
    }
}

pub fn extract(input: &str) -> Option<Vec<ScalaClassImportRequest>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(
            r"^(.*\.scala).*error: Symbol 'type ([A-Za-z0-9.<>_]+)' is missing from the classpath.$"
        )
        .unwrap();
    }

    let mut result = None;
    for ln in input.lines() {
        let captures = RE.captures(ln);

        match captures {
            None => (),
            Some(captures) => {
                let src_file_name = captures.get(1).unwrap().as_str();
                let class_name = captures.get(2).unwrap().as_str();
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
    fn test_symbol_is_missing_from_classpath_error() {
        let sample_output ="
src/main/scala/com/example/D.scala:9: error: Symbol 'type com.example.a.ATrait' is missing from the classpath.
This symbol is required by 'trait com.example.b.BTraitExtendsA'.
Make sure that type ATrait is in your classpath and check for conflicting dependencies with `-Ylog-classpath`.
A full rebuild may help if 'BTraitExtendsA.class' was compiled against an incompatible version of com.example.a.
object DClass extends com.example.c.CTraitExtendsBTraitExtendsATrait
                                    ^
one error found
one error found";

        assert_eq!(
            extract(sample_output),
            Some(vec![build_class_import_request(
                String::from("src/main/scala/com/example/D.scala"),
                "com.example.a.ATrait".to_string()
            )])
        );
    }
}
