use std::collections::HashSet;

use super::super::error_extraction;
use error_extraction::ClassImportRequest;
pub(in crate::bazel_runner) fn sanitize_label(label: String) -> String {
    // If you use macros, say the scala_library suite or similar
    // to generate many rule invocations from one call site, you need to collapse these back
    // for us to be able to add deps/action things.

    let label = match label.find("_auto_gen_") {
        None => label,
        Some(idx) => label[0..idx].to_string(),
    };

    // Here we are normalizing
    // src/foo/bar/baz and src/foo/bar/baz:baz
    // ensures we don't try refer to ourselves

    let label = match label.find(":") {
        None => {
            let last_segment = &label[label.rfind("/").map(|e| e + 1).unwrap_or(0)..label.len()];
            format!("{}:{}", label, last_segment).to_string()
        }
        Some(_) => label,
    };

    label
}

pub(in crate::bazel_runner) fn prepare_class_import_requests(
    mut class_import_requests: Vec<ClassImportRequest>,
) -> Vec<ClassImportRequest> {
    // if a more specific reference to a class/package exists which covers the same package space
    // and that one is allowed recursive search. Then remove the less specific ones, since we will fall back to those
    // via the more specific anyway.

    // First we identify which targets are allowed recursive search.
    let mut recursive_enabled = HashSet::new();
    for e in class_import_requests.iter() {
        if !e.exact_only {
            recursive_enabled.insert(e.class_name.clone());
        }
    }

    // next we prune the existing import requests
    let mut i = 0;
    while i != class_import_requests.len() {
        let element = &class_import_requests[i];
        let mut found = false;
        for recur in recursive_enabled.iter() {
            if recur.contains(&element.class_name) && (*recur) != element.class_name {
                found = true;
                break;
            }
        }

        if found {
            class_import_requests.remove(i);
        } else {
            i += 1;
        }
    }
    class_import_requests
}

fn split_clazz_to_lst(class_name: &str) -> Vec<String> {
    let mut long_running_string = String::new();
    let mut result = Vec::new();
    let mut loop_cnt = 0;
    class_name.split(".").for_each(|segment| {
        if long_running_string.len() > 0 {
            long_running_string = format!("{}.{}", long_running_string, segment);
        } else {
            long_running_string = segment.to_string();
        }
        // we only allow things more specific than `com.example.foo`
        // otherwise its just too generic and a dice roll.
        if loop_cnt > 2 {
            result.push(long_running_string.to_string())
        }
        loop_cnt += 1;
    });
    // ensure we at least pass back out the original class name.
    if result.len() == 0 {
        result.push(class_name.to_string());
    }
    result.reverse();
    result
}

pub(in crate::bazel_runner) fn expand_candidate_import_requests(
    candidate_import_requests: Vec<ClassImportRequest>,
) -> Vec<(ClassImportRequest, Vec<String>)> {
    let mut candidate_import_requests = prepare_class_import_requests(candidate_import_requests);

    candidate_import_requests.sort_by(|a, b| b.priority.cmp(&a.priority));

    candidate_import_requests
        .into_iter()
        .map(|e| {
            let sub_attempts = if e.exact_only {
                vec![e.class_name.clone()]
            } else {
                split_clazz_to_lst(&e.class_name)
            };
            (e, sub_attempts)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_clazz_to_lst() {
        assert_eq!(
            split_clazz_to_lst("a.b.c.d.e"),
            vec![String::from("a.b.c.d.e"), String::from("a.b.c.d"),]
        );

        assert_eq!(split_clazz_to_lst("abcd"), vec![String::from("abcd"),]);
    }

    #[test]
    fn test_sanitize_label() {
        assert_eq!(
            sanitize_label(String::from("foo_bar")),
            String::from("foo_bar:foo_bar")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz:werwe_auto_gen_werewr")),
            String::from("foo/bar/baz:werwe")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz:foop")),
            String::from("foo/bar/baz:foop")
        );

        assert_eq!(
            sanitize_label(String::from("foo/bar/baz")),
            String::from("foo/bar/baz:baz")
        );
    }

    #[test]
    fn test_prepare_class_import_requests() {
        let input = vec![
            ClassImportRequest {
                class_name: String::from("asdf.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
        ];

        //pass through, no change
        assert_eq!(
            prepare_class_import_requests(input),
            vec![
                ClassImportRequest {
                    class_name: String::from("asdf.sadf.sdfwer.sdf"),
                    exact_only: false,
                    src_fn: String::from("unused"),
                    priority: 1
                },
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                    exact_only: false,
                    src_fn: String::from("unused"),
                    priority: 1,
                }
            ]
        );

        // subset prune
        let input = vec![
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
        ];

        // only the longer one is kept
        assert_eq!(
            prepare_class_import_requests(input),
            vec![ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1
            },]
        );

        // cannot prune since set to exact only
        let input = vec![
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                exact_only: true,
                src_fn: String::from("unused"),
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
        ];

        // only the longer one is kept
        assert_eq!(
            prepare_class_import_requests(input),
            vec![
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf.sdfwer.sdf"),
                    exact_only: true,
                    src_fn: String::from("unused"),
                    priority: 1,
                },
                ClassImportRequest {
                    class_name: String::from("foo_bar_baz.sadf"),
                    exact_only: false,
                    src_fn: String::from("unused"),
                    priority: 1,
                },
            ]
        );
    }

    #[test]
    fn test_expand_candidate_import_requests() {
        let input = vec![
            ClassImportRequest {
                class_name: String::from("asdf.sadf.sdfwer.sdf.adsf.wer"),
                exact_only: false,
                src_fn: String::from("unused"),
                priority: 1,
            },
            ClassImportRequest {
                class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                exact_only: true,
                src_fn: String::from("unused"),
                priority: 100,
            },
        ];

        //pass through, no change
        assert_eq!(
            expand_candidate_import_requests(input),
            vec![
                (
                    ClassImportRequest {
                        class_name: String::from("foo_bar_baz.sadf.sdfwer.sdfee"),
                        exact_only: true,
                        src_fn: String::from("unused"),
                        priority: 100,
                    },
                    vec![String::from("foo_bar_baz.sadf.sdfwer.sdfee"),]
                ),
                (
                    ClassImportRequest {
                        class_name: String::from("asdf.sadf.sdfwer.sdf.adsf.wer"),
                        exact_only: false,
                        src_fn: String::from("unused"),
                        priority: 1
                    },
                    vec![
                        String::from("asdf.sadf.sdfwer.sdf.adsf.wer"),
                        String::from("asdf.sadf.sdfwer.sdf.adsf"),
                        String::from("asdf.sadf.sdfwer.sdf"),
                    ]
                )
            ]
        );
    }
}
