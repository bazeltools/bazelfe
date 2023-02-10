fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute("Digest", "#[derive(Hash, Eq, PartialOrd, Ord)]")
        .type_attribute("FileStatus", "#[derive(Hash, Eq, PartialOrd, Ord)]")
        .type_attribute("Instant", "#[derive(Hash, Eq, PartialOrd, Ord)]")
        .compile(
            &[
                "proto/upstream_other/build_event_stream/build_event_stream.proto",
                "proto/upstream_other/blaze_query/build.proto",
                "proto/upstream_other/devtools/buildozer/api.proto",
                "proto/googleapis/google/bytestream/bytestream.proto",
                "proto/googleapis/google/devtools/build/v1/publish_build_event.proto",
                "proto/googleapis/google/rpc/code.proto",
                "proto/remote-apis/build/bazel/remote/execution/v2/remote_execution.proto",
                "proto/remote-apis/build/bazel/remote/asset/v1/remote_asset.proto",
                "proto/local_protos/bzl_remote/bazelfe_index.proto",
                "proto/local_protos/bzl_remote/bazelfe_kv.proto",
                "proto/local_protos/bzl_remote/metadata_service.proto",
                "proto/local_protos/bazel_tools/daemon_service/daemon_service.proto",
                "proto/local_protos/bazel_tools/request_files_service.proto",
                "proto/local_protos/bazel_tools/upstream_service.proto",
            ],
            &[
                "proto/local_protos",
                "proto/upstream_other",
                "proto/remote-apis",
                "proto/googleapis",
            ],
        )?;

    Ok(())
}
