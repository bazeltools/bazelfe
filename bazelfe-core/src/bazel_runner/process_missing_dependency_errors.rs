use bazelfe_protos::*;
use std::{collections::{HashMap, HashSet}, path::Path, path::PathBuf};

use lazy_static::lazy_static;

use crate::{build_events::hydrated_stream::ActionFailedErrorInfo, buildozer_driver::Buildozer, error_extraction, index_table};

use dashmap::DashSet;
use log;


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

      println!("Uh.. huh {:?}", label);
      if let Some(forbidden_targets) = target_kind.as_ref().and_then(|nme| FORBIDDEN_TARGETS_BY_TYPE.get(nme)) {
          if forbidden_targets.contains(label) {
              return false;
          }
    }
            

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
            let candidates = match &req {
                Request::Prefix(class_name) => 
                
                index_table
        .get_or_guess(class_name)
        .await,
                Request::Suffix(suffix) => {
                    index_table.get_from_suffix( &suffix.suffix).await
                }
            };
            for target_entry in &candidates.read_iter().await {
                debug!("Processing candidate for class name: {:#?} : {:#?}", req, target_entry);

                if !ignore_dep_references.contains(&target_entry.target)
                    && is_potentially_valid_target(&action_failed_error_info.target_kind,&target_entry.target)
                {
                    // If our top candidate hits to be a local previous seen stop
                    // processing this class
                    if local_previous_seen.contains(&target_entry.target) {
                        break 'class_entry_loop;
                    }

                    // otherwise... add the dependency with buildozer here
                    // then add it ot the local seen dependencies
                    info!(
                        "Buildozer action: add dependency {:?} to {:?}",
                        target_entry.target, action_failed_error_info.label
                    );
                    buildozer
                        .add_dependency(&action_failed_error_info.label, &target_entry.target)
                        .await
                        .unwrap();
                    actions_completed += 1;

                    local_previous_seen.insert(target_entry.target.clone());

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
    use std::{sync::Arc, path::PathBuf};

            use tokio::sync::Mutex;

use crate::{buildozer_driver::ExecuteResultError, error_extraction::{ClassImportRequest, ClassSuffixMatch}};

use super::*;



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
        assert_eq!(is_potentially_valid_target(&Some(String::from("scala_library")), "@third_party_jvm//3rdparty/jvm/org/scala_lang:scala_library"), false);
    }


    #[test]
    fn test_output_error_paths() {
        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(String::from("file:///foo/bar/baz"))
            ],
            target_kind: Some(String::from("scala_library"))
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
        expected_suffix_requests: Vec<error_extraction::ClassSuffixMatch>
    ) {
        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(String::from("file:///foo/bar/baz"))
            ],
            target_kind: Some(String::from(target_kind))
        };

        let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
        tempfile.write_all(content.as_bytes()).expect("Should be able to write to temp file");
        let tempfile_path = tempfile.into_temp_path();

        let mut candidate_import_requests: Vec<error_extraction::ClassImportRequest> = Vec::default();
        let mut suffix_requests: Vec<error_extraction::ClassSuffixMatch> = Vec::default();
        path_to_import_requests(
            &action_failed_error_info,
            &(*tempfile_path).to_path_buf(),
            &mut candidate_import_requests,
            &mut suffix_requests,
        ).await;

        assert_eq!(candidate_import_requests, expected_candidate_import_requests);
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
                vec![
                    ClassSuffixMatch { suffix: String::from("JSONObject"), src_fn: String::from("java::error_cannot_access")}
                ]
            ).await
    }


    // Scenarios we need to test for processing missing dependency errors:
    // -> have some of our targets in previously seen
    // -> buildozer failed
    // -> print deps say we already have the action
    // -> for one target, we have multiple class errors. For some of those errors we should share the first hit.
    
    #[tokio::test]
    async fn test_process_missing_dependency_errors() {

        // this is a simple scenario, nothing is in the index table, and we have our buildozer set to allow ~everything to pass through

        let buildozer = FakeBuildozer{
            action_log: Arc::new(Mutex::new(Vec::default()))
        };

        let content = "src/main/scala/com/example/Example.scala:2: error: object foo is not a member of package com.example
        import com.example.foo.bar.Baz
                           ^
        src/main/scala/com/example/Example.scala:2: warning: Unused import
        import com.example.foo.bar.Baz
                                   ^
        one warning found
        one error found";

        let mut tempfile = tempfile::NamedTempFile::new().expect("Can make a temp file");
        tempfile.write_all(content.as_bytes()).expect("Should be able to write to temp file");
        let tempfile_path = tempfile.into_temp_path();

        let action_failed_error_info = ActionFailedErrorInfo {
            label: String::from("//src/main/com/example/foo:Bar"),
            output_files: vec![
                build_event_stream::file::File::Uri(String::from("remote_uri://foo/bar/baz")),
                build_event_stream::file::File::Uri(format!("file://{}", &(*tempfile_path).to_path_buf().to_str().unwrap().to_string()))
            ],
            target_kind: Some(String::from("scala_library"))
        };

        let index_table = index_table::IndexTable::default();
        let global_previous_seen = DashSet::new();


        let current_dir = std::env::current_dir().unwrap().to_owned();
        
        let working_bazel_tempdir = tempfile::tempdir().expect("Can create tempdir");
        
        std::env::set_current_dir(&working_bazel_tempdir.path()).expect("Can set the cwd");


        // Now we need to setup the state on the disk such that things will work...

        std::fs::create_dir_all(Path::new("src/main/scala/com/example/foo/bar")).unwrap();
        std::fs::write("src/main/scala/com/example/foo/bar/BUILD", "java_librar(...)").expect("Should be able to write file");

        std::fs::create_dir_all(Path::new("src/main/scala/com/example/foo")).unwrap();
        std::fs::write("src/main/scala/com/example/foo/BUILD", "java_librar(...)").expect("Should be able to write file");


        let actions = process_missing_dependency_errors(
            &global_previous_seen,
            buildozer.clone(),
            &action_failed_error_info,
            &index_table,
        ).await;

        std::env::set_current_dir(&current_dir).expect("Can set the cwd");

        assert_eq!(actions, 1);

        let event_log: Vec<ActionLogEntry> = buildozer.to_vec().await;

        let expected_action_log: Vec<ActionLogEntry> = vec![
            ActionLogEntry::AddDependency { target_to_operate_on: String::from("//src/main/com/example/foo:Bar"), label_to_add: String::from("//src/main/scala/com/example/foo:foo") }
        ];
        assert_eq!(
            event_log,
            expected_action_log
        );

    }
    #[derive(Clone, Debug, PartialEq)]
    enum ActionLogEntry {
        AddDependency{
            target_to_operate_on: String,
            label_to_add: String
        },
        RemovedDependency{
            target_to_operate_on: String,
            label_to_add: String
        }
    }
    #[derive(Clone, Debug)]
    struct FakeBuildozer {
        action_log: Arc<Mutex<Vec<ActionLogEntry>>>
    }
    impl FakeBuildozer {
        pub async fn to_vec(&self) -> Vec<ActionLogEntry> {
            let locked = self.action_log.lock().await;
            (*locked).clone()
        }
    }

    #[async_trait::async_trait]
    impl Buildozer for FakeBuildozer {
        async fn print_deps(&self, label: &String) -> Result<Vec<String>, ExecuteResultError> {
            Ok(Vec::default())
        }
        async fn add_dependency(
            &self,
            target_to_operate_on: &String,
            label_to_add: &String,
        ) -> Result<(), ExecuteResultError> {
            let mut lock = self.action_log.lock().await;
            lock.push(ActionLogEntry::AddDependency{
                target_to_operate_on: target_to_operate_on.clone(),
                label_to_add: label_to_add.clone(),
            });
            Ok(())
        }
    
        async fn remove_dependency(
            &self,
            target_to_operate_on: &String,
            label_to_add: &String,
        ) -> Result<(), ExecuteResultError> {
            let mut lock = self.action_log.lock().await;
            lock.push(ActionLogEntry::RemovedDependency{
                target_to_operate_on: target_to_operate_on.clone(),
                label_to_add: label_to_add.clone(),
            });
            Ok(())
        }
    }
    
}
