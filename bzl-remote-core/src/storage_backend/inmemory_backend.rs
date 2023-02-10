use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use dashmap::DashMap;

use std::sync::Arc;

use super::api::insert_action_result_to_cas;
use super::api::UploadType;
use super::StorageBackend;
use super::StorageBackendError;

#[derive(PartialEq, Hash, Eq, Debug)]
struct ByteWrapper(Vec<u8>);

impl std::borrow::Borrow<[u8]> for ByteWrapper {
    fn borrow(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(Debug)]
pub struct InMemoryStorageBackend {
    kv_map: Arc<DashMap<Vec<u8>, Vec<u8>>>,
    action_map: Arc<DashMap<execution::Digest, Arc<execution::ActionResult>>>,
    cas_store: Arc<DashMap<ByteWrapper, Arc<Vec<u8>>>>,
}

impl Default for InMemoryStorageBackend {
    fn default() -> Self {
        Self {
            kv_map: Arc::new(DashMap::default()),
            action_map: Arc::new(DashMap::default()),
            cas_store: Arc::new(DashMap::default()),
        }
    }
}

impl InMemoryStorageBackend {}

pub struct ArcRefBox(Arc<Vec<u8>>);
impl AsRef<[u8]> for ArcRefBox {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[async_trait::async_trait]
impl StorageBackend for InMemoryStorageBackend {
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        Ok(self.kv_map.get(key).map(|v| v.clone()))
    }

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        self.kv_map.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError> {
        Ok(self.action_map.get(digest).map(|v| Arc::clone(v.value())))
    }
    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError> {
        let action_result_digest = insert_action_result_to_cas(self, action_result).await?;

        self.action_map
            .insert(digest.clone(), Arc::new(action_result.clone()));
        Ok(action_result_digest)
    }

    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        digests.retain(|ele| {
            let lut = ele.hash.as_bytes();
            !self.cas_store.contains_key(lut)
        });
        Ok(())
    }

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        let lut = hash.as_bytes();
        if let Some(d) = self.cas_store.get(lut) {
            Ok(Some(execution::Digest {
                hash: hash.clone(),
                size_bytes: d.len() as i64,
            }))
        } else {
            Ok(None)
        }
    }

    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        let lut = digest.hash.as_bytes();
        Ok(self.cas_store.contains_key(lut))
    }

    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError> {
        let data = match data {
            UploadType::InMemory(data) => data,
            UploadType::OnDisk(path) => {
                use std::io::Read;
                let mut f = std::fs::File::open(&path)
                    .map_err(|e| StorageBackendError::InternalError(Box::new(e)))?;
                let mut contents = Vec::new();
                f.read_to_end(&mut contents)
                    .map_err(|e| StorageBackendError::InternalError(Box::new(e)))?;
                let _ = std::fs::remove_file(&path); // ignore errors in removing temp files.
                contents
            }
        };
        self.cas_store
            .insert(ByteWrapper(digest.hash.as_bytes().to_vec()), Arc::new(data));
        Ok(())
    }

    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Box<dyn AsRef<[u8]> + Send>>, StorageBackendError> {
        let lut = digest.hash.as_bytes();

        Ok(self.cas_store.get(lut).map(|e| {
            let v = Arc::clone(&e);
            let b: Box<dyn AsRef<[u8]> + Send> = Box::new(ArcRefBox(v));
            b
        }))
    }
}
