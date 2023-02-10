use super::cache_service_config::CacheServiceConfig;
use serde::{Deserialize, Deserializer};

#[derive(Deserialize, Debug, PartialEq, Eq)]
pub struct Config {
    #[serde(rename = "CacheServiceConfig", default = "CacheServiceConfig::default")]
    pub cache_config: CacheServiceConfig,

    /// where to bind the local port to listen to the grpc service
    #[serde(default, deserialize_with = "parse_bind_address")]
    pub bind_address: Option<std::net::SocketAddr>,

    #[serde(default = "default_send_buffer_size")]
    pub send_buffer_size: usize,
}

fn default_send_buffer_size() -> usize {
    4194304 - 1024
}

// We want to use the serde configured defaults for our default implemenation to not be
// building up two separate paths.
impl Default for Config {
    fn default() -> Self {
        super::parse_config("").unwrap()
    }
}

fn parse_bind_address<'de, D>(deserializer: D) -> Result<Option<std::net::SocketAddr>, D::Error>
where
    D: Deserializer<'de>,
{
    let s: Option<&str> = Deserialize::deserialize(deserializer)?;

    if let Some(s) = s {
        s.parse().map_err(serde::de::Error::custom).map(Some)
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {

    use crate::config::cache_service_config::CloudBackendConfig;

    use super::*;
    #[test]
    fn test_in_memory() {
        let config: Config = toml::from_str(
            r#"
        [[CacheServiceConfig]]
          cache_backend = { type = 'InMemory'  }
          type = 'InMemory'
        "#,
        )
        .unwrap();

        assert_eq!(
            config.cache_config.cache_backend,
            super::super::cache_service_config::CacheServiceStorage::InMemory {}
        );
    }

    #[test]
    fn test_on_disk() {
        let config: Config = toml::from_str(
            r#"
        [[CacheServiceConfig]]
          path = '/foo/bar/storage'
          type = 'OnLocalDisk'
        "#,
        )
        .unwrap();

        assert_eq!(
            config.cache_config.cache_backend,
            super::super::cache_service_config::CacheServiceStorage::OnLocalDisk {
                path: std::path::PathBuf::from("/foo/bar/storage")
            }
        );
    }

    #[test]
    fn test_cloud() {
        let config: Config = toml::from_str(
            r#"
        [[CacheServiceConfig]]
          local_working_path = '/foo/bar/storage'
          redis_host = 'redisserver.foo.bar'
          s3_region = 'us-east-1'
          s3_bucket = 'mycustombucket'
          s3_prefix = 'test_stuff_a'
          type = 'CloudBackend'
        "#,
        )
        .unwrap();

        assert_eq!(
            config.cache_config.cache_backend,
            super::super::cache_service_config::CacheServiceStorage::CloudBackend(
                CloudBackendConfig {
                    redis_host: "redisserver.foo.bar".to_string(),
                    s3_region: "us-east-1".to_string(),
                    s3_bucket: "mycustombucket".to_string(),
                    s3_prefix: "test_stuff_a".to_string(),
                    local_working_path: String::from("/foo/bar/storage")
                }
            )
        );
    }

    #[test]
    fn test_empty_parse() {
        let config: Config = toml::from_str("").unwrap();

        assert_eq!(
            config.cache_config.cache_backend,
            super::super::cache_service_config::CacheServiceStorage::InMemory {}
        );
    }
}
