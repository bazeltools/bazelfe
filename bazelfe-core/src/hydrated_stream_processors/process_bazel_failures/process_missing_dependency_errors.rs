use std::{
    collections::{HashMap, HashSet},
    path::Path,
    path::PathBuf,
    time::Instant,
};

use lazy_static::lazy_static;

use crate::{
    build_events::hydrated_stream::ActionFailedErrorInfo,
    buildozer_driver::Buildozer,
    error_extraction::{self, ActionRequest},
    index_table,
};

use super::CurrentState;

use super::shared_utils::output_error_paths;

fn is_potentially_valid_target(target_kind: &Option<String>, label: &str) -> bool {
    lazy_static! {
      // These are things that are already implicit dependencencies so we should ensure they are not included
        static ref FORBIDDEN_TARGETS_BY_TYPE: HashMap<String, HashSet<String>> = {
            let mut m = HashMap::new();
            let mut cur_s = HashSet::new();
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library",
            ));
            m.insert(String::from("scala_library"), cur_s);

            let mut cur_s = HashSet::new();
            cur_s.insert(String::from("@third_party_jvm//3rdparty/jvm/org/scalatest"));
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scalatest:scalatest",
            ));
            cur_s.insert(String::from(
                "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library",
            ));
            m.insert(String::from("scala_test"), cur_s);
            m
        };
    }

    if let Some(forbidden_targets) = target_kind
        .as_ref()
        .and_then(|nme| FORBIDDEN_TARGETS_BY_TYPE.get(nme))
    {
        if forbidden_targets.contains(label) {
            return false;
        }
    }

    let prepared_path = label.strip_prefix("//").and_then(|e| e.split(":").next());
    match prepared_path {
        Some(p) => {
            let path = Path::new(p);
            path.join("BUILD").exists() || path.join("BUILD.bazel").exists()
        }
        None => true,
    }
}

async fn path_to_import_requests(
    error_info: &ActionFailedErrorInfo,
    path_to_use: &PathBuf,
    action_requests: &mut Vec<ActionRequest>,
) {
    let loaded_path = tokio::fs::read_to_string(path_to_use).await.unwrap();

    action_requests.extend(error_extraction::extract_errors(
        &error_info.target_kind,
        &loaded_path,
    ));
}

pub async fn load_up_ignore_references<T: Buildozer + Clone + Send + Sync + 'static>(
    global_previous_seen: &mut HashSet<String>,
    buildozer: &T,
    action_failed_error_info: &ActionFailedErrorInfo,
) -> HashSet<String> {
    let mut to_ignore = HashSet::new();
    let d = buildozer
        .print_deps(&action_failed_error_info.label)
        .await
        .unwrap();
    d.into_iter().for_each(|dep| {
        to_ignore.insert(crate::label_utils::sanitize_label(dep));
    });

    global_previous_seen.iter().for_each(|dep| {
        to_ignore.insert(crate::label_utils::sanitize_label(dep.to_string()));
    });

    to_ignore.insert(crate::label_utils::sanitize_label(
        action_failed_error_info.label.clone(),
    ));

    global_previous_seen.insert(crate::label_utils::sanitize_label(
        action_failed_error_info.label.clone(),
    ));

    to_ignore
}

pub fn expand_candidate_import_requests(action_requests: Vec<ActionRequest>) -> Vec<ActionRequest> {
    let mut res_action_requests = Vec::default();
    let mut candidate_import_requests = Vec::default();

    for e in action_requests {
        match e {
            ActionRequest::Prefix(p) => candidate_import_requests.push(p),
            ActionRequest::Suffix(s) => res_action_requests.push(ActionRequest::Suffix(s)),
        }
    }

    let mut candidate_import_requests =
        crate::label_utils::prepare_class_import_requests(candidate_import_requests);

    let mut extras = Vec::default();

    for c in candidate_import_requests.iter() {
        if !c.exact_only {
            let r = crate::label_utils::class_name_to_prefixes(&c.class_name);
            let len = r.len();
            for (offset, sub_pre) in r.into_iter().enumerate() {
                let inner_offset = ((len - offset) as i32) * -1;
                extras.push(error_extraction::ClassImportRequest {
                    class_name: sub_pre,
                    priority: -50 + c.priority + inner_offset,
                    exact_only: true,
                    src_fn: c.src_fn.clone(),
                });
            }
        }
    }

    candidate_import_requests.extend(extras.into_iter());

    for y in candidate_import_requests.into_iter() {
        res_action_requests.push(ActionRequest::Prefix(y));
    }

    res_action_requests.sort();
    res_action_requests.dedup();

    res_action_requests
}

async fn generate_all_action_requests(
    action_failed_error_info: &ActionFailedErrorInfo,
) -> Vec<ActionRequest> {
    let mut action_requests: Vec<ActionRequest> = vec![];
    for path in output_error_paths(&action_failed_error_info).into_iter() {
        path_to_import_requests(
            &action_failed_error_info,
            &path.into(),
            &mut action_requests,
        )
        .await
    }
    expand_candidate_import_requests(action_requests)
}
pub async fn process_missing_dependency_errors<T: Buildozer>(
    current_state: &mut CurrentState,
    buildozer: T,
    action_failed_error_info: &ActionFailedErrorInfo,
    index_table: &index_table::IndexTable,
    epoch: usize,
) -> super::Response {
    if epoch <= current_state.epoch {
        return super::Response::new(Vec::default());
    }
    let ignore_dep_references: HashSet<String> = load_up_ignore_references(
        &mut current_state.ignore_list,
        &buildozer,
        action_failed_error_info,
    )
    .await;
    let all_requests: Vec<ActionRequest> =
        generate_all_action_requests(&action_failed_error_info).await;
    debug!("generate_all_action_requests: {:#?}", all_requests);
    let (response, local_previous_seen, remove_from_ignore_list) =
        inner_process_missing_dependency_errors(
            buildozer,
            &action_failed_error_info.label,
            &action_failed_error_info.target_kind,
            index_table,
            all_requests,
            ignore_dep_references,
            &mut current_state.added_target_for_class,
        )
        .await;

    // concat the global perm ignore with the local_previous seen data
    // this becomes our next global ignore for this target
    for e in local_previous_seen.into_iter() {
        current_state.ignore_list.insert(e);
    }
    for e in remove_from_ignore_list.into_iter() {
        current_state.ignore_list.remove(&e);
    }

    current_state.epoch = epoch;
    response
}
async fn inner_process_missing_dependency_errors<T: Buildozer>(
    buildozer: T,
    label: &str,
    target_kind: &Option<String>,
    index_table: &index_table::IndexTable,
    all_requests: Vec<ActionRequest>,
    ignore_dep_references: HashSet<String>,
    previous_added: &mut HashMap<ActionRequest, HashSet<String>>,
) -> (super::Response, HashSet<String>, HashSet<String>) {
    let mut local_previous_seen: HashSet<String> = HashSet::new();
    let mut local_previous_seen_prefix: HashSet<String> = HashSet::new();

    let mut to_remove: HashSet<String> = HashSet::new();
    let mut target_stories = Vec::default();
    let unsanitized_label = label;
    let label = crate::label_utils::sanitize_label(String::from(label));

    let mut total_added = 0;
    'req_point: for req in all_requests.into_iter() {
        let candidates = match &req {
            ActionRequest::Suffix(suffix) => index_table.get_from_suffix(&suffix.suffix).await,
            ActionRequest::Prefix(prefix) => {
                if local_previous_seen_prefix.contains(&prefix.class_name) {
                    continue 'req_point;
                } else {
                    local_previous_seen_prefix.insert(prefix.class_name.clone());
                }
                // We dont guess if its exact only. Proxy for lower confidence.

                if prefix.exact_only {
                    index_table
                        .get(&prefix.class_name)
                        .await
                        .unwrap_or_default()
                } else {
                    index_table.get_or_guess(&prefix.class_name).await
                }
            }
        };
        let why = match &req {
            ActionRequest::Prefix(prefix) => format!(
                "Saw missing dependency: prefix/class: {}, for: {}",
                &prefix.class_name, &prefix.src_fn
            ),
            ActionRequest::Suffix(s) => format!(
                "Saw missing dependency:  suffix match: {}, for: {}",
                s.suffix, s.src_fn
            ),
        };

        let previous_added_for_req = match previous_added.get_mut(&req) {
            Some(req) => req,
            None => {
                previous_added.insert(req.clone(), Default::default());
                previous_added.get_mut(&req).unwrap()
            }
        };

        for prev in previous_added_for_req.iter() {
            let prev_deps = buildozer.print_deps(&label).await.unwrap();

            if prev_deps.contains(&prev) {
                debug!(
                    "Buildozer action: remove dependency {:?} to {:?}",
                    prev, &label
                );
                buildozer.remove_dependency(&label, &prev).await.unwrap();

                target_stories.push(super::TargetStory {
                    target: unsanitized_label.to_string(),
                    action: super::TargetStoryAction::RemovedDependency {
                        removed_what: prev.clone(),
                        why: format!(
                            "Removed previously added dependency that didn't solve issue. on: {:?}",
                            req
                        ),
                    },
                    when: Instant::now(),
                });

                to_remove.insert(prev.clone());
            } else {
                debug!(
                    "Was going to remove: {}  from {} but isn't in the deps anymore so no op",
                    prev, label
                );
            }
        }

        let mut target_to_add = None;
        for target_entry in &candidates.read_iter().await {
            let target: String = index_table
                .decode_string(target_entry.target)
                .await
                .unwrap();
            if !ignore_dep_references.contains(&target)
                && is_potentially_valid_target(&target_kind, &target)
            {
                // If our top candidate hits to be a local previous seen stop
                // processing this class
                if local_previous_seen.contains(&target) {
                    continue 'req_point;
                }

                if previous_added_for_req.contains(&target) {
                    continue 'req_point;
                }

                if target_to_add.is_none() {
                    target_to_add = Some(target.clone());
                }
            }
        }
        if let Some(target) = target_to_add {
            // otherwise... add the dependency with buildozer here
            // then add it ot the local seen dependencies
            debug!(
                "Buildozer action: add dependency {:?} to {:?}",
                target, &label
            );
            previous_added_for_req.insert(target.clone());

            total_added += 1;
            buildozer.add_dependency(&label, &target).await.unwrap();
            target_stories.push(super::TargetStory {
                target: unsanitized_label.to_string(),
                action: super::TargetStoryAction::AddedDependency {
                    added_what: target.clone(),
                    why: why.clone(),
                },
                when: Instant::now(),
            });

            local_previous_seen.insert(target.clone());
            if total_added < 5 {
                continue 'req_point;
            } else {
                break 'req_point;
            }
        }
    }

    (
        super::Response::new(target_stories),
        local_previous_seen,
        to_remove,
    )
}

#[cfg(test)]
mod tests {
    use bazelfe_protos::*;

    use once_cell::sync::Lazy;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::Mutex;

    use crate::{
        buildozer_driver::ExecuteResultError,
        error_extraction::{ActionRequest, ClassImportRequest, ClassSuffixMatch},
    };

    use super::*;

    static RELIES_ON_CWD: Lazy<Mutex<()>> = Lazy::new(Mutex::default);

    #[test]
    fn test_is_potentially_valid_target() {
        assert_eq!(is_potentially_valid_target(&None, "@foo/bar/baz"), true);

        assert_eq!(is_potentially_valid_target(&None, "//foo/bar/foo"), false);

        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/bazel/sample_build");
        let built_path = format!("//{}", d.to_str().unwrap());
        assert_eq!(is_potentially_valid_target(&None, &built_path), true);
    }

    #[test]
    fn test_is_potentially_valid_target_forbidden_by_type() {
        assert_eq!(
            is_potentially_valid_target(
                &Some(String::from("scala_library")),
                "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library"
            ),
            false
        );
    }

    #[test]
    fn test_output_error_paths() {
        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            stderr: Some(build_event_stream::File {
                name: String::default(),
                path_prefix: Vec::default(),
                file: Some(build_event_stream::file::File::Uri(String::from(
                    "remote_uri://foo/bar/baz",
                ))),
            }),

            stdout: Some(build_event_stream::File {
                name: String::default(),
                path_prefix: Vec::default(),
                file: Some(build_event_stream::file::File::Uri(String::from(
                    "file:///foo/bar/baz",
                ))),
            }),
            target_kind: Some(String::from("scala_library")),
        };

        let result: Vec<PathBuf> = output_error_paths(&action_failed_error_info);
        let expected: Vec<PathBuf> = vec![Path::new("/foo/bar/baz").to_path_buf()];
        assert_eq!(result, expected);
    }
    use std::io::prelude::*;

    async fn test_content_to_expected_result(
        content: &str,
        target_kind: &str,
        expected_candidate_import_requests: Vec<error_extraction::ClassImportRequest>,
        expected_suffix_requests: Vec<error_extraction::ClassSuffixMatch>,
    ) {
        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            stdout: None,
            stderr: None,
            target_kind: Some(String::from(target_kind)),
        };

        let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
        tempfile
            .write_all(content.as_bytes())
            .expect("Should be able to write to temp file");
        let tempfile_path = tempfile.into_temp_path();

        let mut action_requests: Vec<ActionRequest> = Vec::default();
        path_to_import_requests(
            &action_failed_error_info,
            &(*tempfile_path).to_path_buf(),
            &mut action_requests,
        )
        .await;

        let mut candidate_import_requests: Vec<error_extraction::ClassImportRequest> =
            Vec::default();
        let mut suffix_requests: Vec<error_extraction::ClassSuffixMatch> = Vec::default();
        for e in action_requests.into_iter() {
            match e {
                ActionRequest::Prefix(p) => {
                    candidate_import_requests.push(p);
                }
                ActionRequest::Suffix(p) => {
                    suffix_requests.push(p);
                }
            }
        }

        assert_eq!(
            candidate_import_requests,
            expected_candidate_import_requests
        );
        assert_eq!(suffix_requests, expected_suffix_requests);
    }

    #[tokio::test]
    async fn test_path_to_import_requests() {
        // we are just testing that we load the file and invoke the paths, so we just need ~any error types in here.
        test_content_to_expected_result(
            "src/main/scala/com/example/Example.scala:2: error: object foo is not a member of package com.example
            import com.example.foo.bar.Baz
                               ^
            src/main/scala/com/example/Example.scala:2: warning: Unused import
            import com.example.foo.bar.Baz
                                       ^
            one warning found
            one error found",
            "scala_library",
            vec![ClassImportRequest{
                class_name: String::from("com.example.foo"), exact_only: false, src_fn: String::from("scala::extract_not_a_member_of_package"), priority: 5
            }],
            Vec::default()
        ).await;
    }

    #[tokio::test]
    async fn test_generate_all_action_requests() {
        async fn test_content_to_expected_result(
            content: &str,
            target_kind: &str,
            expected_requests: Vec<ActionRequest>,
        ) {
            let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
            tempfile
                .write_all(content.as_bytes())
                .expect("Should be able to write to temp file");
            let tempfile_path = tempfile.into_temp_path();

            let action_failed_error_info = ActionFailedErrorInfo {
                label: String::from("//src/main/com/example/foo:Bar"),
                stderr: Some(build_event_stream::File {
                    name: String::default(),
                    path_prefix: Vec::default(),
                    file: Some(build_event_stream::file::File::Uri(String::from(
                        "remote_uri://foo/bar/baz",
                    ))),
                }),

                stdout: Some(build_event_stream::File {
                    name: String::default(),
                    path_prefix: Vec::default(),
                    file: Some(build_event_stream::file::File::Uri(format!(
                        "file://{}",
                        &(*tempfile_path).to_path_buf().to_str().unwrap().to_string()
                    ))),
                }),

                target_kind: Some(String::from(target_kind)),
            };

            let generated_requests = generate_all_action_requests(&action_failed_error_info).await;

            assert_eq!(generated_requests, expected_requests);
        }

        // we are just testing that we load the file and invoke the paths, so we just need ~any error types in here.
        test_content_to_expected_result(
            "src/main/scala/com/example/Example.scala:2: error: object foo is not a member of package com.example
            import com.example.foo.bar.Baz
                               ^
            src/main/scala/com/example/Example.scala:2: warning: Unused import
            import com.example.foo.bar.Baz
                                       ^
            one warning found
            one error found",
            "scala_library",

                vec![ActionRequest::Prefix(
                    ClassImportRequest {
                    class_name: String::from("com.example.foo"),
    exact_only: false,
    src_fn: String::from("scala::extract_not_a_member_of_package"),
    priority: 5})]
        ).await;

        // we are just testing that we load the file and invoke the paths, so we just need ~any error types in here.
        test_content_to_expected_result(
            "src/main/java/com/example/foo/bar/Baz.java:205: error: cannot access JSONObject
                        Blah key = Blah.myfun(jwk);

        src/main/java/com/example/foo/Example.java:16: error: cannot find symbol
            import javax.annotation.foo.bar.baz.Nullable;
                                   ^
              symbol:   class Nullable
              location: package javax.annotation.foo.bar.baz",
            "java_library",
            vec![
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("javax.annotation.foo.bar.baz.Nullable"),
                    exact_only: false,
                    src_fn: String::from("java::cannot_find_symbol"),
                    priority: 1,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("javax.annotation.foo.bar.baz"),
                    exact_only: true,
                    src_fn: String::from("java::cannot_find_symbol"),
                    priority: -50,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("javax.annotation.foo.bar"),
                    exact_only: true,
                    src_fn: String::from("java::cannot_find_symbol"),
                    priority: -51,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("javax.annotation.foo"),
                    exact_only: true,
                    src_fn: String::from("java::cannot_find_symbol"),
                    priority: -52,
                }),
                ActionRequest::Suffix(ClassSuffixMatch {
                    suffix: String::from("JSONObject"),
                    src_fn: String::from("java::error_cannot_access"),
                }),
            ],
        )
        .await
    }

    // Scenarios we need to test for processing missing dependency errors:
    // -> have some of our targets in previously seen
    // -> buildozer failed
    // -> print deps say we already have the action
    // -> for one target, we have multiple class errors. For some of those errors we should share the first hit.

    #[tokio::test]
    async fn test_process_missing_dependency_errors() {
        let _lock = RELIES_ON_CWD.lock().await;

        // this is a simple scenario, nothing is in the index table, and we have our buildozer set to allow ~everything to pass through

        let buildozer = FakeBuildozer::default();

        let content = "src/main/scala/com/example/Example.scala:2: error: object foo is not a member of package com.example
        import com.example.foo.bar.Baz
                           ^
        src/main/scala/com/example/Example.scala:2: warning: Unused import
        import com.example.foo.bar.Baz
                                   ^
        one warning found
        one error found";

        let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
        tempfile
            .write_all(content.as_bytes())
            .expect("Should be able to write to temp file");
        let tempfile_path = tempfile.into_temp_path();

        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),

            stderr: Some(build_event_stream::File {
                name: String::default(),
                path_prefix: Vec::default(),
                file: Some(build_event_stream::file::File::Uri(String::from(
                    "remote_uri://foo/bar/baz",
                ))),
            }),

            stdout: Some(build_event_stream::File {
                name: String::default(),
                path_prefix: Vec::default(),
                file: Some(build_event_stream::file::File::Uri(format!(
                    "file://{}",
                    &(*tempfile_path).to_path_buf().to_str().unwrap().to_string()
                ))),
            }),

            target_kind: Some(String::from("scala_library")),
        };

        let index_table = index_table::IndexTable::default();
        let mut global_previous_seen = CurrentState::default();

        let current_dir = std::env::current_dir().unwrap().to_owned();

        let working_bazel_tempdir = tempfile::tempdir().expect("Can create tempdir");

        std::env::set_current_dir(&working_bazel_tempdir.path()).expect("Can set the cwd");

        // Now we need to setup the state on the disk such that things will work...

        std::fs::create_dir_all(Path::new("src/main/scala/com/example/foo/bar"))
            .expect("Can create directories");
        std::fs::write(
            "src/main/scala/com/example/foo/bar/BUILD",
            "java_librar(...)",
        )
        .expect("Should be able to write file");

        std::fs::create_dir_all(Path::new("src/main/scala/com/example/foo")).unwrap();
        std::fs::write("src/main/scala/com/example/foo/BUILD", "java_librar(...)")
            .expect("Should be able to write file");

        let response = process_missing_dependency_errors(
            &mut global_previous_seen,
            buildozer.clone(),
            &action_failed_error_info,
            &index_table,
            1,
        )
        .await;

        std::env::set_current_dir(&current_dir).expect("Can set the cwd");

        assert_eq!(response.target_story_entries.len(), 1);

        let event_log: Vec<ActionLogEntry> = buildozer.to_vec().await;

        let expected_action_log: Vec<ActionLogEntry> = vec![ActionLogEntry::AddDependency {
            target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
            label_to_add: String::from("//src/main/scala/com/example/foo:foo"),
        }];
        assert_eq!(event_log, expected_action_log);
    }

    #[tokio::test]
    async fn test_inner_process_missing_dependency_errors() {
        let _lock = RELIES_ON_CWD.lock().await;
        async fn run_scenario(
            paths_to_exist: Vec<&str>,
            index_table: index_table::IndexTable,
            ignore_dep_references: HashSet<String>,
            buildozer: FakeBuildozer,
            all_requests: Vec<ActionRequest>,
        ) -> (Vec<ActionLogEntry>, super::super::Response) {
            // this is a simple scenario, nothing is in the index table, and we have our buildozer set to allow ~everything to pass through

            let current_dir = std::env::current_dir().unwrap().to_owned();

            let working_bazel_tempdir = tempfile::tempdir().expect("Can create tempdir");

            std::env::set_current_dir(&working_bazel_tempdir.path())
                .expect("Unable to set the CWD to the test folder");

            // Now we need to setup the state on the disk such that things will work...

            for path in paths_to_exist {
                std::fs::create_dir_all(Path::new(path))
                    .expect("Should be able to make directories");
                std::fs::write(format!("{}/BUILD", path), "java_librar(...)")
                    .expect("Should be able to write file");
            }

            let mut previous_added = HashMap::default();
            let (response, _, _) = inner_process_missing_dependency_errors(
                buildozer.clone(),
                "//src/main/com/example/foo:Bar",
                &Some(String::from("scala_library")),
                &index_table,
                all_requests,
                ignore_dep_references,
                &mut previous_added,
            )
            .await;

            std::env::set_current_dir(&current_dir)
                .expect("Unable to set the CWD back to the repo");

            let event_log: Vec<ActionLogEntry> = buildozer.to_vec().await;

            (event_log, response)
        }

        // No requests
        let (action_log_entry, response) = run_scenario(
            vec!["src/main/scala/com/example/foo"],
            index_table::IndexTable::default(),
            HashSet::new(),
            FakeBuildozer::default(),
            vec![],
        )
        .await;
        assert_eq!(response.target_story_entries.len(), 0);
        assert_eq!(action_log_entry.len(), 0);

        // Action request, and nothing in the index table.
        // have the path on disk
        let (action_log_entry, response) = run_scenario(
            vec![
                "src/main/scala/com/example/foo",
                "src/main/scala/com/example/foo/bar/baz",
            ],
            index_table::IndexTable::default(),
            HashSet::new(),
            FakeBuildozer::default(),
            vec![ActionRequest::Prefix(ClassImportRequest {
                class_name: String::from("com.example.foo.bar.baz"),
                exact_only: false,
                src_fn: String::from("scala::extract_not_a_member_of_package"),
                priority: 5,
            })],
        )
        .await;
        assert_eq!(response.target_story_entries.len(), 1);
        assert_eq!(
            action_log_entry,
            vec![ActionLogEntry::AddDependency {
                target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
                label_to_add: String::from("//src/main/scala/com/example/foo/bar/baz:baz"),
            }]
        );

        // two independent classes needed.
        // should add both
        let (action_log_entry, response) = run_scenario(
            vec![
                "src/main/scala/com/example/foo",
                "src/main/scala/com/example/foo/bar/baz",
                "src/main/scala/com/example/foo/bar/noof",
            ],
            index_table::IndexTable::default(),
            HashSet::new(),
            FakeBuildozer::default(),
            vec![
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("com.example.foo.bar.baz"),
                    exact_only: false,
                    src_fn: String::from("scala::extract_not_a_member_of_package"),
                    priority: 5,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("com.example.foo.bar.noof"),
                    exact_only: false,
                    src_fn: String::from("scala::extract_not_a_member_of_package"),
                    priority: 5,
                }),
            ],
        )
        .await;
        assert_eq!(response.target_story_entries.len(), 2);
        assert_eq!(
            action_log_entry,
            vec![
                ActionLogEntry::AddDependency {
                    target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
                    label_to_add: String::from("//src/main/scala/com/example/foo/bar/baz:baz"),
                },
                ActionLogEntry::AddDependency {
                    target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
                    label_to_add: String::from("//src/main/scala/com/example/foo/bar/noof:noof"),
                }
            ]
        );

        // Same set of requests as above, but we are going to stick baz:baz into our global have visited list, thus
        // it should be ignored.
        let mut ignore_dep_references = HashSet::new();
        ignore_dep_references.insert(String::from("//src/main/scala/com/example/foo/bar/baz:baz"));
        let (action_log_entry, response) = run_scenario(
            vec![
                "src/main/scala/com/example/foo",
                "src/main/scala/com/example/foo/bar/baz",
                "src/main/scala/com/example/foo/bar/noof",
            ],
            index_table::IndexTable::default(),
            ignore_dep_references,
            FakeBuildozer::default(),
            vec![
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("com.example.foo.bar.baz"),
                    exact_only: false,
                    src_fn: String::from("scala::extract_not_a_member_of_package"),
                    priority: 5,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("com.example.foo.bar.baz"),
                    exact_only: false,
                    src_fn: String::from("scala::extract_not_a_member_of_package"),
                    priority: 5,
                }),
                ActionRequest::Prefix(ClassImportRequest {
                    class_name: String::from("com.example.foo.bar.noof"),
                    exact_only: false,
                    src_fn: String::from("scala::extract_not_a_member_of_package"),
                    priority: 5,
                }),
            ],
        )
        .await;
        assert_eq!(response.target_story_entries.len(), 1);
        assert_eq!(
            action_log_entry,
            vec![ActionLogEntry::AddDependency {
                target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
                label_to_add: String::from("//src/main/scala/com/example/foo/bar/noof:noof"),
            }]
        );
    }
    #[derive(Clone, Debug, PartialEq)]
    enum ActionLogEntry {
        AddDependency {
            target_to_operate_on: String,
            label_to_add: String,
        },
        RemovedDependency {
            target_to_operate_on: String,
            label_to_add: String,
        },
    }
    #[derive(Clone, Debug)]
    struct FakeBuildozer {
        action_log: Arc<Mutex<Vec<ActionLogEntry>>>,
        add_dependency_pairs_to_fail: HashSet<(String, String)>,
        remove_dependency_pairs_to_fail: HashSet<(String, String)>,
    }
    impl Default for FakeBuildozer {
        fn default() -> Self {
            FakeBuildozer::new(HashSet::new(), HashSet::new())
        }
    }
    impl FakeBuildozer {
        pub fn new(
            add_dependency_pairs_to_fail: HashSet<(String, String)>,
            remove_dependency_pairs_to_fail: HashSet<(String, String)>,
        ) -> Self {
            Self {
                action_log: Arc::new(Mutex::new(Vec::default())),
                add_dependency_pairs_to_fail: add_dependency_pairs_to_fail,
                remove_dependency_pairs_to_fail: remove_dependency_pairs_to_fail,
            }
        }
        pub async fn to_vec(&self) -> Vec<ActionLogEntry> {
            let locked = self.action_log.lock().await;
            (*locked).clone()
        }
    }

    #[async_trait::async_trait]
    impl Buildozer for FakeBuildozer {
        async fn print_deps(&self, _label: &String) -> Result<Vec<String>, ExecuteResultError> {
            Ok(Vec::default())
        }
        async fn add_dependency(
            &self,
            target_to_operate_on: &str,
            label_to_add: &String,
        ) -> Result<(), ExecuteResultError> {
            if self
                .add_dependency_pairs_to_fail
                .contains(&(String::from(target_to_operate_on), label_to_add.clone()))
            {
                return Err(ExecuteResultError {
                    exit_code: -1,
                    stdout: String::default(),
                    stderr: String::default(),
                });
            }
            let mut lock = self.action_log.lock().await;
            lock.push(ActionLogEntry::AddDependency {
                target_to_operate_on: target_to_operate_on.to_string(),
                label_to_add: label_to_add.clone(),
            });
            Ok(())
        }

        async fn remove_dependency(
            &self,
            target_to_operate_on: &String,
            label_to_add: &String,
        ) -> Result<(), ExecuteResultError> {
            if self
                .remove_dependency_pairs_to_fail
                .contains(&(String::from(target_to_operate_on), label_to_add.clone()))
            {
                return Err(ExecuteResultError {
                    exit_code: -1,
                    stdout: String::default(),
                    stderr: String::default(),
                });
            }

            let mut lock = self.action_log.lock().await;
            lock.push(ActionLogEntry::RemovedDependency {
                target_to_operate_on: target_to_operate_on.clone(),
                label_to_add: label_to_add.clone(),
            });
            Ok(())
        }
    }
}
