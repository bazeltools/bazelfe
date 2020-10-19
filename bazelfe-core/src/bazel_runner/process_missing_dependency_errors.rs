use bazelfe_protos::*;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    path::PathBuf,
};

use lazy_static::lazy_static;

use crate::{
    build_events::hydrated_stream::ActionFailedErrorInfo, buildozer_driver::Buildozer,
    error_extraction, index_table,
};

use dashmap::DashSet;
use log;

fn get_candidates_for_class_name(
    error_info: &ActionFailedErrorInfo,
    class_name: &str,
    index_table: &index_table::IndexTable,
) -> Vec<(u16, String)> {
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

    let mut results = index_table
        .get(class_name)
        .map(|e| e.clone())
        .unwrap_or(vec![]);

    match &error_info.target_kind {
        Some(target_kind) => match FORBIDDEN_TARGETS_BY_TYPE.get(target_kind) {
            None => (),
            Some(forbidden_targets) => {
                results = results
                    .into_iter()
                    .filter(|(_, target)| !forbidden_targets.contains(target))
                    .collect();
            }
        },
        None => (),
    };

    results = results
        .into_iter()
        .chain(super::expand_target_to_guesses::get_guesses_for_class_name(class_name).into_iter())
        .map(|(a, b)| (a, super::sanitization_tools::sanitize_label(b)))
        .collect();

    results.sort_by(|a, b| b.0.cmp(&a.0));
    results
}

pub fn is_potentially_valid_target(label: &str) -> bool {
    let prepared_path = label.strip_prefix("//").and_then(|e| e.split(":").next());
    match prepared_path {
        Some(p) => {
            let path = Path::new(p);
            path.join("BUILD").exists()
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

pub async fn process_missing_dependency_errors<T: Buildozer + Clone + Send + Sync + 'static>(
    global_previous_seen: &DashSet<String>,
    buildozer: T,
    action_failed_error_info: &ActionFailedErrorInfo,
    index_table: &index_table::IndexTable,
) -> u32 {
    let mut local_previous_seen: HashSet<String> = HashSet::new();

    let ignore_dep_references: HashSet<String> = {
        let mut to_ignore = HashSet::new();
        let d = buildozer
            .print_deps(&action_failed_error_info.label)
            .await
            .unwrap();
        d.into_iter().for_each(|dep| {
            to_ignore.insert(super::sanitization_tools::sanitize_label(dep));
        });

        global_previous_seen.iter().for_each(|dep| {
            to_ignore.insert(super::sanitization_tools::sanitize_label(dep.to_string()));
        });

        to_ignore.insert(super::sanitization_tools::sanitize_label(
            action_failed_error_info.label.clone(),
        ));

        global_previous_seen.insert(super::sanitization_tools::sanitize_label(
            action_failed_error_info.label.clone(),
        ));

        to_ignore
    };
    log::debug!("ignore_dep_references: {:?}", ignore_dep_references);

    let mut actions_completed: u32 = 0;

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

    debug!("Prefix Candidates: {:#?}", prefix_candidate_import_requests);
    #[derive(Debug, PartialEq)]
    enum Request {
        Prefix(String),
        Suffix(error_extraction::ClassSuffixMatch),
    }

    let all_requests: Vec<Vec<Request>> = Box::new(
        super::sanitization_tools::expand_candidate_import_requests(
            prefix_candidate_import_requests,
        )
        .into_iter()
        .map(|(_, inner)| {
            inner
                .into_iter()
                .map(|e| Request::Prefix(e))
                .collect::<Vec<Request>>()
        }),
    )
    .chain(
        suffix_requests
            .into_iter()
            .map(|e| vec![Request::Suffix(e)]),
    )
    .collect();

    for req in all_requests.into_iter() {
        'class_entry_loop: for req in req.into_iter() {
            let candidates: Vec<(u16, String)> = match &req {
                Request::Prefix(class_name) => get_candidates_for_class_name(
                    action_failed_error_info,
                    &class_name,
                    &index_table,
                ),
                Request::Suffix(suffix) => {
                    let mut r = index_table.get_from_suffix(&suffix.suffix);
                    r.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                    r
                }
            };
            debug!("Candidates for class name: {:#?} : {:#?}", req, candidates);
            for (_, target_name) in candidates {
                if !ignore_dep_references.contains(&target_name)
                    && is_potentially_valid_target(&target_name)
                {
                    // If our top candidate hits to be a local previous seen stop
                    // processing this class
                    if local_previous_seen.contains(&target_name) {
                        break 'class_entry_loop;
                    }

                    // otherwise... add the dependency with buildozer here
                    // then add it ot the local seen dependencies
                    info!(
                        "Buildozer action: add dependency {:?} to {:?}",
                        target_name, action_failed_error_info.label
                    );
                    buildozer
                        .add_dependency(&action_failed_error_info.label, &target_name)
                        .await
                        .unwrap();
                    actions_completed += 1;

                    local_previous_seen.insert(target_name.clone());

                    // Now that we have a version with a match we can jump right out to the outside
                    break 'class_entry_loop;
                }
            }
        }
    }

    // concat the global perm ignore with the local_previous seen data
    // this becomes our next global ignore for this target
    for e in local_previous_seen.into_iter() {
        global_previous_seen.insert(e);
    }

    actions_completed
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn get_candidates_from_map() {
        let mut tbl_map = HashMap::new();
        tbl_map.insert(
            String::from("com.example.foo.bar.Baz"),
            vec![(13, String::from("//src/main/foop/blah:oop"))],
        );
        let index_table = index_table::IndexTable::from_hashmap(tbl_map);

        let error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/foo/asd/we:wer"),
            output_files: vec![],
            target_kind: Some(String::from("scala_library")),
        };

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.bar.Baz", &index_table),
            vec![
                (0, String::from("//src/main/scala/com/example/bar:bar")),
                (0, String::from("//src/main/java/com/example/bar:bar")),
            ]
        );

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.foo.bar.Baz", &index_table),
            vec![
                (13, String::from("//src/main/foop/blah:oop")),
                (0, String::from("//src/main/scala/com/example/foo/bar:bar")),
                (0, String::from("//src/main/java/com/example/foo/bar:bar"))
            ]
        );

        assert_eq!(
            get_candidates_for_class_name(&error_info, "com.example.a.b.c.Baz", &index_table),
            vec![
                (0, String::from("//src/main/scala/com/example/a/b/c:c")),
                (0, String::from("//src/main/java/com/example/a/b/c:c"))
            ]
        );
    }

    #[test]
    fn test_is_potentially_valid_target() {
        assert_eq!(is_potentially_valid_target("@foo/bar/baz"), true);

        assert_eq!(is_potentially_valid_target("//foo/bar/foo"), false);

        let mut d = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        d.push("resources/tests/bazel/sample_build");
        let built_path = format!("//{}", d.to_str().unwrap());
        assert_eq!(is_potentially_valid_target(&built_path), true);
    }
}
