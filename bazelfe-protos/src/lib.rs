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
            pub mod asset {
                pub mod v1 {
                    tonic::include_proto!("build.bazel.remote.asset.v1");
                }
            }
        }
    }
}

pub mod blaze_query {
    tonic::include_proto!("blaze_query");
}

pub mod devtools {
    pub mod buildozer {
        tonic::include_proto!("devtools.buildozer");
    }
}

pub mod bzl_remote {
    pub mod bazelfe_index {
        tonic::include_proto!("bzl_remote.bazelfe_index");
    }
    pub mod bazelfe_kv {
        tonic::include_proto!("bzl_remote.bazelfe_kv");
    }
    pub mod metadata_service {
        tonic::include_proto!("bzl_remote.metadata_service");
    }
}

pub mod bazel_tools {
    pub mod daemon_service {
        tonic::include_proto!("bazel_tools.daemon_service");
        impl Copy for Instant {}

        pub trait TargetUtils {
            fn target_label(&self) -> &str;
            fn is_test(&self) -> bool;
        }
        impl TargetUtils for Target {
            fn is_test(&self) -> bool {
                if let Some(res) = self.target_response.as_ref() {
                    match res {
                        target::TargetResponse::BuildLabel(_) => false,
                        target::TargetResponse::TestLabel(_) => true,
                    }
                } else {
                    false
                }
            }

            fn target_label(&self) -> &str {
                if let Some(res) = self.target_response.as_ref() {
                    match res {
                        target::TargetResponse::BuildLabel(label) => label.as_str(),
                        target::TargetResponse::TestLabel(label) => label.as_str(),
                    }
                } else {
                    ""
                }
            }
        }
    }
    tonic::include_proto!("bazel_tools");
}

pub mod digest_utils;
