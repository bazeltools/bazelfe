fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &["proto/build_event_stream/build_event_stream.proto"],
        &["proto", "proto/googleapis"],
    )?;

    // build event
    tonic_build::configure().compile(
        &[
            "proto/googleapis/google/devtools/build/v1/publish_build_event.proto",
            "proto/googleapis/google/bytestream/bytestream.proto",
        ],
        &["proto/googleapis"],
    )?;

    tonic_build::configure().compile(
        &["proto/remote-apis/build/bazel/remote/execution/v2/remote_execution.proto"],
        &["proto/remote-apis", "proto/googleapis"],
    )?;

    tonic_build::configure()
        .type_attribute("FileStatus", "#[derive(Hash, Eq, PartialOrd, Ord)]")
        .type_attribute("Instant", "#[derive(Hash, Eq, PartialOrd, Ord)]")
        .compile(
            &["proto/bazel_tools/daemon_service/daemon_service.proto"],
            &["proto/googleapis", "proto/bazel_tools/daemon_service"],
        )?;

    tonic_build::configure().compile(
        &[
            "proto/bazel_tools/request_files_service.proto",
            "proto/bazel_tools/upstream_service.proto",
        ],
        &["proto/remote-apis", "proto/bazel_tools", "proto/googleapis"],
    )?;

    tonic_build::configure().compile(&["proto/devtools/buildozer/api.proto"], &["proto"])?;

    tonic_build::configure().compile(&["proto/blaze_query/build.proto"], &["proto"])?;

    Ok(())
}
