use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
use execution::capabilities_server;

use bazelfe_protos::*;

use tonic::Request;

pub struct CapabilitiesService {}

impl CapabilitiesService {
    pub fn new() -> CapabilitiesService {
        CapabilitiesService {}
    }
}

impl Default for CapabilitiesService {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl capabilities_server::Capabilities for CapabilitiesService {
    async fn get_capabilities(
        &self,
        request: Request<execution::GetCapabilitiesRequest>,
    ) -> Result<tonic::Response<execution::ServerCapabilities>, tonic::Status> {
        tracing::debug!("Got Capabilites request: {:#?}", request);
        Ok(tonic::Response::new(execution::ServerCapabilities {
            cache_capabilities: Some(execution::CacheCapabilities {
                digest_function: vec![execution::digest_function::Value::Sha256 as i32],
                action_cache_update_capabilities: Some(execution::ActionCacheUpdateCapabilities {
                    update_enabled: true,
                }),
                cache_priority_capabilities: Some(execution::PriorityCapabilities {
                    priorities: vec![execution::priority_capabilities::PriorityRange {
                        min_priority: -100,
                        max_priority: 100,
                    }],
                }),
                max_batch_total_size_bytes: 4 * 1024 * 1024,
                symlink_absolute_path_strategy:
                    execution::symlink_absolute_path_strategy::Value::Disallowed as i32,
            }),
            execution_capabilities: None,
            deprecated_api_version: None,
            low_api_version: Some(build::bazel::semver::SemVer {
                major: 2,
                ..Default::default()
            }),
            high_api_version: Some(build::bazel::semver::SemVer {
                major: 2,
                ..Default::default()
            }),
        }))
    }
}

#[cfg(test)]
mod tests {

    use super::super::OptionAsStatusError;
    use super::*;

    use capabilities_server::Capabilities;
    #[tokio::test]
    async fn test_results_are_expected() -> Result<(), Box<dyn std::error::Error>> {
        let svc = CapabilitiesService::new();

        let capabilities_response = svc
            .get_capabilities(Request::new(execution::GetCapabilitiesRequest {
                instance_name: String::from("Anything"),
            }))
            .await
            .expect("Should succeed to get capabilities");

        let mut capabilities_response = capabilities_response.into_inner();

        // Take out this field, it should be present, mutates the response, but convient way to inspect it
        let cache_capabilities = capabilities_response.cache_capabilities.take_or_error()?;
        assert_eq!(cache_capabilities.digest_function.len(), 1);
        assert_eq!(
            cache_capabilities.digest_function[0],
            execution::digest_function::Value::Sha256 as i32
        );

        Ok(())
    }
}
