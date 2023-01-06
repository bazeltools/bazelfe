pub mod index_new_results;
pub mod process_bazel_failures;

#[derive(Clone, Debug)]
pub enum BuildEventResponse {
    ProcessedBuildFailures(process_bazel_failures::Response),
    IndexedResults(index_new_results::Response),
}
