use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

mod content_addressable_store;
use content_addressable_store::ContentAddressableStore;

mod action_cache;
mod kv_store;
use action_cache::ActionCache;

use std::path::Path;

use std::sync::Arc;

pub use self::content_addressable_store::DataLocation;
use self::kv_store::KvStore;

use super::api::insert_action_result_to_cas;
use super::api::DataReturnTpe;
use super::StorageBackend;
use super::StorageBackendError;
use super::UploadType;

#[derive(Debug)]
pub struct LocalDiskStorageBackend {
    content_addressable_store: ContentAddressableStore,
    action_cache: ActionCache,
    kv_store: KvStore,
}

impl LocalDiskStorageBackend {
    // This should probably change to being a setup error return type
    // but given this is done during the open flow only, going to short cut and use a dyn Error for now.
    // Lossy in the type.
    pub fn open<P: AsRef<Path>>(p: P) -> Result<Self, Box<dyn std::error::Error>> {
        let root_path = p.as_ref();
        std::fs::create_dir_all(root_path)?;

        let db_folder = root_path.join("database");
        std::fs::create_dir_all(&db_folder)?;

        let large_blob = root_path.join("large_blob");
        std::fs::create_dir_all(&large_blob)?;

        let sled_connection = sled::open(db_folder.join("sled"))?;

        let action_cache = ActionCache::new(&sled_connection)?;
        let kv_store = KvStore::new(&sled_connection, "kv_store")?;
        let content_addressable_store = ContentAddressableStore::new(&sled_connection, large_blob)?;

        Ok(Self {
            action_cache,
            content_addressable_store,
            kv_store,
        })
    }

    pub async fn insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<DataLocation, StorageBackendError> {
        self.content_addressable_store.insert(digest, data).await
    }

    pub async fn cas_to_vec(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Vec<u8>>, StorageBackendError> {
        self.content_addressable_store.get_to_vec(digest).await
    }
}

#[async_trait::async_trait]
impl StorageBackend for LocalDiskStorageBackend {
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        Ok(self.kv_store.get(key)?.map(|e| e.as_ref().to_vec()))
    }

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        self.kv_store.insert(key, value)?;
        Ok(())
    }

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError> {
        self.action_cache
            .get_action(digest)
            .map(|e| e.map(Arc::new))
    }
    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError> {
        let action_result_digest = insert_action_result_to_cas(self, action_result).await?;
        self.action_cache.insert(digest, action_result)?;
        Ok(action_result_digest)
    }

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        self.content_addressable_store
            .build_digest_from_hash_if_present(hash)
            .await
    }

    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        let mut res: Vec<execution::Digest> = Vec::with_capacity(digests.len());
        for ele in digests.drain(..) {
            if !self.content_addressable_store.exists(&ele)? {
                res.push(ele);
            }
        }
        std::mem::swap(digests, &mut res);

        Ok(())
    }

    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        self.content_addressable_store.exists(digest)
    }

    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError> {
        self.insert(digest, data).await?;
        Ok(())
    }

    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataReturnTpe>, StorageBackendError> {
        if let Some(r) = self.content_addressable_store.get(digest).await? {
            Ok(Some(Box::new(r)))
        } else {
            Ok(None)
        }
    }
}
