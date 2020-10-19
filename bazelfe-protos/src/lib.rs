pub mod google {
    pub mod devtools {
        pub mod build {
            pub mod v1 {
                tonic::include_proto!("google.devtools.build.v1");
            }
        }
    }
    pub mod bytestream {
        tonic::include_proto!("google.bytestream");
    }
    pub mod rpc {
        tonic::include_proto!("google.rpc");
    }
    pub mod longrunning {
        tonic::include_proto!("google.longrunning");
    }
}
pub mod blaze {
    pub mod invocation_policy {
        tonic::include_proto!("blaze.invocation_policy");
    }
}

pub mod failure_details {
    tonic::include_proto!("failure_details");
}

pub mod options {
    tonic::include_proto!("options");
}

pub mod command_line {
    tonic::include_proto!("command_line");
}

pub mod build_event_stream {
    tonic::include_proto!("build_event_stream");
}

pub mod build {
    pub mod bazel {
        pub mod semver {
            tonic::include_proto!("build.bazel.semver");
        }
        pub mod remote {
            pub mod execution {
                pub mod v2 {
                    tonic::include_proto!("build.bazel.remote.execution.v2");
                }
            }
        }
    }
}

pub mod bazel_tools {
    tonic::include_proto!("bazel_tools");
}

pub mod devtools {
    pub mod buildozer {
        tonic::include_proto!("devtools.buildozer");
    }
}
