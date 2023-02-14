use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type")]
pub enum CacheServiceStorage {
    OnLocalDisk { path: PathBuf },
    InMemory {},
    CloudBackend(CloudBackendConfig),
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct CloudBackendConfig {
    pub redis_host: String,
    pub s3_region: String,
    pub s3_bucket: String,
    pub s3_prefix: String,
    pub local_working_path: String,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Clone)]
pub struct CacheServiceConfig {
    #[serde(default = "cache_backend")]
    pub cache_backend: CacheServiceStorage,
}

impl Default for CacheServiceConfig {
    fn default() -> Self {
        toml::from_str("").unwrap()
    }
}

fn cache_backend() -> CacheServiceStorage {
    CacheServiceStorage::InMemory {}
}

#[cfg(test)]
mod tests {}
