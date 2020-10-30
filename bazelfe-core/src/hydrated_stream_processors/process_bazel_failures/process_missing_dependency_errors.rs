use bazelfe_protos::*;
use std::{time::Instant, collections::{HashMap, HashSet}, path::Path, path::PathBuf};

use lazy_static::lazy_static;

use crate::{
    build_events::hydrated_stream::ActionFailedErrorInfo, buildozer_driver::Buildozer,
    error_extraction, index_table,
};

use dashmap::DashSet;

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

fn output_error_paths(err_data: &ActionFailedErrorInfo) -> Vec<std::path::PathBuf> {
    err_data
        .output_files
        .iter()
        .flat_map(|e| match e {
            build_event_stream::file::File::Uri(e) => {
                if e.starts_with("file://") {
                    let u: PathBuf = e.strip_prefix("file://").unwrap().into();
                    Some(u)
                } else {
                    warn!("Path isn't a file, so skipping...{:?}", e);

                    None
                }
            }
            build_event_stream::file::File::Contents(_) => None,
        })
        .collect()
}

async fn path_to_import_requests(
    error_info: &ActionFailedErrorInfo,
    path_to_use: &PathBuf,
    candidate_import_requests: &mut Vec<error_extraction::ClassImportRequest>,
    suffix_requests: &mut Vec<error_extraction::ClassSuffixMatch>,
) {
    let loaded_path = tokio::fs::read_to_string(path_to_use).await.unwrap();

    candidate_import_requests.extend(error_extraction::extract_errors(
        &error_info.target_kind,
        &loaded_path,
    ));
    suffix_requests.extend(error_extraction::extract_suffix_errors(
        &error_info.target_kind,
        &loaded_path,
    ));
}

pub async fn load_up_ignore_references<T: Buildozer + Clone + Send + Sync + 'static>(
    global_previous_seen: &DashSet<String>,
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

#[derive(Debug, PartialEq)]
enum ActionRequest {
    Prefix(String),
    Suffix(error_extraction::ClassSuffixMatch),
}

async fn generate_all_action_requests(
    action_failed_error_info: &ActionFailedErrorInfo,
) -> Vec<Vec<ActionRequest>> {
    let mut prefix_candidate_import_requests: Vec<error_extraction::ClassImportRequest> = vec![];
    let mut suffix_requests: Vec<error_extraction::ClassSuffixMatch> = vec![];
    for path in output_error_paths(&action_failed_error_info).into_iter() {
        path_to_import_requests(
            &action_failed_error_info,
            &path.into(),
            &mut prefix_candidate_import_requests,
            &mut suffix_requests,
        )
        .await
    }

    Box::new(
        crate::label_utils::expand_candidate_import_requests(
            prefix_candidate_import_requests,
        )
        .into_iter()
        .map(|(_, inner)| {
            inner
                .into_iter()
                .map(|e| ActionRequest::Prefix(e))
                .collect::<Vec<ActionRequest>>()
        }),
    )
    .chain(
        suffix_requests
            .into_iter()
            .map(|e| vec![ActionRequest::Suffix(e)]),
    )
    .collect()
}
pub async fn process_missing_dependency_errors<T: Buildozer>(
    global_previous_seen: &DashSet<String>,
    buildozer: T,
    action_failed_error_info: &ActionFailedErrorInfo,
    index_table: &index_table::IndexTable,
) -> super::Response {
    let ignore_dep_references: HashSet<String> =
        load_up_ignore_references(global_previous_seen, &buildozer, action_failed_error_info).await;
    let all_requests: Vec<Vec<ActionRequest>> =
        generate_all_action_requests(&action_failed_error_info).await;
    let (response, local_previous_seen) = inner_process_missing_dependency_errors(
        buildozer,
        &action_failed_error_info.label,
        &action_failed_error_info.target_kind,
        index_table,
        all_requests,
        ignore_dep_references,
    )
    .await;

    // concat the global perm ignore with the local_previous seen data
    // this becomes our next global ignore for this target
    for e in local_previous_seen.into_iter() {
        global_previous_seen.insert(e);
    }
    response
}
async fn inner_process_missing_dependency_errors<T: Buildozer>(
    buildozer: T,
    label: &str,
    target_kind: &Option<String>,
    index_table: &index_table::IndexTable,
    all_requests: Vec<Vec<ActionRequest>>,
    ignore_dep_references: HashSet<String>,
) -> (super::Response, HashSet<String>) {
    let mut local_previous_seen: HashSet<String> = HashSet::new();
    let mut target_stories = Vec::default();
    let unsanitized_label = label;
    let label = crate::label_utils::sanitize_label(String::from(label));

    for req in all_requests.into_iter() {
        'class_entry_loop: for req in req.into_iter() {
            let candidates = match &req {
                ActionRequest::Prefix(class_name) => index_table.get_or_guess(class_name).await,
                ActionRequest::Suffix(suffix) => index_table.get_from_suffix(&suffix.suffix).await,
            };
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
                        break 'class_entry_loop;
                    }

                    // otherwise... add the dependency with buildozer here
                    // then add it ot the local seen dependencies
                    info!(
                        "Buildozer action: add dependency {:?} to {:?}",
                        target, &label
                    );
                    buildozer.add_dependency(&label, &target).await.unwrap();
                    target_stories.push(
                        super::TargetStory{
                            target: unsanitized_label.to_string(),
                            action: super::TargetStoryAction::AddedDependency{
                                added_what: target.clone(),
                                why: String::from("Saw a missing dependency error"),
                            },
                            when: Instant::now(),
                        }
                    );

                    local_previous_seen.insert(target.clone());

                    // Now that we have a version with a match we can jump right out to the outside
                    break 'class_entry_loop;
                }
            }
        }
    }

    (super::Response::new(target_stories.len() as u32, target_stories), local_previous_seen)
}

#[cfg(test)]
mod tests {
    use once_cell::sync::Lazy;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::Mutex;

    use crate::{
        buildozer_driver::ExecuteResultError,
        error_extraction::{ClassImportRequest, ClassSuffixMatch},
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
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(String::from("file:///foo/bar/baz")),
            ],
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
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(String::from("file:///foo/bar/baz")),
            ],
            target_kind: Some(String::from(target_kind)),
        };

        let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
        tempfile
            .write_all(content.as_bytes())
            .expect("Should be able to write to temp file");
        let tempfile_path = tempfile.into_temp_path();

        let mut candidate_import_requests: Vec<error_extraction::ClassImportRequest> =
            Vec::default();
        let mut suffix_requests: Vec<error_extraction::ClassSuffixMatch> = Vec::default();
        path_to_import_requests(
            &action_failed_error_info,
            &(*tempfile_path).to_path_buf(),
            &mut candidate_import_requests,
            &mut suffix_requests,
        )
        .await;

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
                class_name: String::from("com.example.foo"), exact_only: false, src_fn: String::from("scala::extract_not_a_member_of_package"), priority: 1
            }],
            Vec::default()
        ).await;

        // we are just testing that we load the file and invoke the paths, so we just need ~any error types in here.
        test_content_to_expected_result(
            "src/main/java/com/example/foo/bar/Baz.java:205: error: cannot access JSONObject
                Blah key = Blah.myfun(jwk);",
            "java_library",
            Vec::default(),
            vec![ClassSuffixMatch {
                suffix: String::from("JSONObject"),
                src_fn: String::from("java::error_cannot_access"),
            }],
        )
        .await
    }

    #[tokio::test]
    async fn test_generate_all_action_requests() {
        async fn test_content_to_expected_result(
            content: &str,
            target_kind: &str,
            expected_requests: Vec<Vec<ActionRequest>>,
        ) {
            let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
            tempfile
                .write_all(content.as_bytes())
                .expect("Should be able to write to temp file");
            let tempfile_path = tempfile.into_temp_path();

            let action_failed_error_info = ActionFailedErrorInfo {
                label: String::from("//src/main/com/example/foo:Bar"),
                output_files: vec![
                    build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                    build_event_stream::file::File::Uri(format!(
                        "file://{}",
                        &(*tempfile_path).to_path_buf().to_str().unwrap().to_string()
                    )),
                ],
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
            vec![
                vec![ActionRequest::Prefix(String::from("com.example.foo"))]
            ],
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
                vec![
                    ActionRequest::Prefix(String::from("javax.annotation.foo.bar.baz.Nullable")),
                    ActionRequest::Prefix(String::from("javax.annotation.foo.bar.baz")),
                    ActionRequest::Prefix(String::from("javax.annotation.foo.bar")),
                ],
                vec![ActionRequest::Suffix(ClassSuffixMatch {
                    suffix: String::from("JSONObject"),
                    src_fn: String::from("java::error_cannot_access"),
                })],
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
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(format!(
                    "file://{}",
                    &(*tempfile_path).to_path_buf().to_str().unwrap().to_string()
                )),
            ],
            target_kind: Some(String::from("scala_library")),
        };

        let index_table = index_table::IndexTable::default();
        let global_previous_seen = DashSet::new();

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
            &global_previous_seen,
            buildozer.clone(),
            &action_failed_error_info,
            &index_table,
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
            all_requests: Vec<Vec<ActionRequest>>,
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

            let (response, _) = inner_process_missing_dependency_errors(
                buildozer.clone(),
                "//src/main/com/example/foo:Bar",
                &Some(String::from("scala_library")),
                &index_table,
                all_requests,
                ignore_dep_references,
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
            vec![vec![ActionRequest::Prefix(String::from(
                "com.example.foo.bar.baz",
            ))]],
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
                vec![ActionRequest::Prefix(String::from(
                    "com.example.foo.bar.baz",
                ))],
                vec![ActionRequest::Prefix(String::from(
                    "com.example.foo.bar.noof",
                ))],
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

        // Two not quite independent requests, via generation
        // we expect that we will add baz:baz for the first request
        // and such we should skip the operation on the second request, doing nothing.
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
                vec![ActionRequest::Prefix(String::from(
                    "com.example.foo.bar.baz",
                ))],
                vec![
                    ActionRequest::Prefix(String::from("com.example.foo.bar.baz")),
                    ActionRequest::Prefix(String::from("com.example.foo.bar.noof")),
                ],
            ],
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
                vec![ActionRequest::Prefix(String::from(
                    "com.example.foo.bar.baz",
                ))],
                vec![
                    ActionRequest::Prefix(String::from("com.example.foo.bar.baz")),
                    ActionRequest::Prefix(String::from("com.example.foo.bar.noof")),
                ],
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

        // Disabled till we re-do error handling to be result? or a catch unwind here could work.
        //     // Here we are going to simulate a buildozer failure, which is a critical failure now.

        //     let mut add_dependency_pairs_to_fail = HashSet::new();
        //     add_dependency_pairs_to_fail.insert((String::from("//src/main/com/example/foo:Bar"), String::from("//src/main/scala/com/example/foo/bar/baz:baz")));
        //     let buildozer = FakeBuildozer::new(
        //         add_dependency_pairs_to_fail,
        //         HashSet::new()
        //     );
        // let (action_log_entry, actions) = run_scenario(
        //     vec![
        //         "src/main/scala/com/example/foo",
        //         "src/main/scala/com/example/foo/bar/baz",
        //         "src/main/scala/com/example/foo/bar/noof",
        //     ],
        //     index_table::IndexTable::default(),
        //     HashSet::new(),
        //     buildozer,
        //     vec![
        //         vec![
        //             ActionRequest::Prefix(String::from("com.example.foo.bar.baz")),
        //             ActionRequest::Prefix(String::from("com.example.foo.bar.noof")),
        //         ],
        //     ],
        // )
        // .await;
        // assert_eq!(actions, 1);
        // assert_eq!(
        //     action_log_entry,
        //     vec![ActionLogEntry::AddDependency {
        //         target_to_operate_on: String::from("//src/main/com/example/foo:Bar"),
        //         label_to_add: String::from("//src/main/scala/com/example/foo/bar/noof:noof"),
        //     }]
        // );
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
