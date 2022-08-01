use std::collections::HashSet;

use crate::error_extraction;
use error_extraction::ClassImportRequest;

pub fn sanitize_label(label: String) -> String {
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

    let label = match label.find(':') {
        None => {
            let last_segment = &label[label.rfind('/').map(|e| e + 1).unwrap_or(0)..label.len()];
            format!("{}:{}", label, last_segment)
        }
        Some(_) => label,
    };

    label
}

pub fn prepare_class_import_requests(
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

pub fn class_name_to_prefixes(class_name: &str, allow_all_minor_domains: bool) -> Vec<String> {
    let mut long_running_string = String::new();
    let mut result = Vec::new();
    let mut loop_cnt = 0;
    let major_domains = vec!["com", "net", "org"];
    let mut is_major_domain_opt = None;
    class_name.split('.').for_each(|segment| {
        let is_major_domain = if let Some(v) = is_major_domain_opt {
            v
        } else {
            let v = major_domains.contains(&segment);
            is_major_domain_opt = Some(v);
            v
        };

        if !long_running_string.is_empty() {
            long_running_string = format!("{}.{}", long_running_string, segment);
        } else {
            long_running_string = segment.to_string();
        }
        // we only allow things more specific than `com.example`
        // otherwise its just too generic and a dice roll for com, net and org.
        // otherwise it likely reflects an organization
        let required_loop_cnt = if is_major_domain {
            1
        } else if allow_all_minor_domains {
            -1
        } else {
            0
        };

        if loop_cnt > required_loop_cnt && long_running_string != class_name {
            result.push(long_running_string.to_string())
        }
        loop_cnt += 1;
    });
    result
}

pub fn expand_candidate_import_requests(
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
                let mut r = class_name_to_prefixes(&e.class_name, false);
                r.push(e.class_name.clone());
                r.reverse();
                r
            };
            (e, sub_attempts)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_class_name_to_prefixes() {
        assert_eq!(
            class_name_to_prefixes("a.b.c.d.e", false),
            vec![
                String::from("a.b"),
                String::from("a.b.c"),
                String::from("a.b.c.d")
            ]
        );

        let expected: Vec<String> = vec![];
        assert_eq!(class_name_to_prefixes("abcd", false), expected);

        assert_eq!(
            class_name_to_prefixes("vegas.sparkExt.package", false),
            vec![String::from("vegas.sparkExt")]
        );

        assert_eq!(class_name_to_prefixes("com.google.foo", false), expected);
        assert_eq!(class_name_to_prefixes("net.google.foo", false), expected);

        assert_eq!(class_name_to_prefixes("org.google.foo", false), expected);
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
                        String::from("asdf.sadf.sdfwer"),
                        String::from("asdf.sadf"),
                    ]
                )
            ]
        );
    }
}
