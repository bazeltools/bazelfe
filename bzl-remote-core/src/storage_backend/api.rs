use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
use prost::Message;

use std::{path::PathBuf, sync::Arc};

use crate::hash::sha256_value::Sha256Value;

use super::StorageBackendError;

#[derive(Debug)]
pub enum UploadType {
    InMemory(Vec<u8>),
    OnDisk(PathBuf),
}

pub type DataReturnTpe = Box<dyn AsRef<[u8]> + Send>;

pub async fn insert_action_result_to_cas<T: StorageBackend>(
    cas: T,
    action_result: &execution::ActionResult,
) -> Result<execution::Digest, StorageBackendError> {
    let action_result_bytes = action_result.encode_to_vec();
    let sha_v: Sha256Value = action_result_bytes.as_slice().try_into()?;
    let action_result_digest = execution::Digest {
        hash: sha_v.to_string(),
        size_bytes: action_result_bytes.len() as i64,
    };
    cas.cas_insert(
        &action_result_digest,
        UploadType::InMemory(action_result_bytes),
    )
    .await?;
    Ok(action_result_digest)
}

#[async_trait::async_trait]
pub trait StorageBackend: Send + Sync + std::fmt::Debug {
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError>;

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError>;

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError>;

    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError>;

    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError>;

    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError>;

    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError>;

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError>;

    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataReturnTpe>, StorageBackendError>;
}

#[async_trait::async_trait]
impl<T> StorageBackend for Arc<T>
where
    T: StorageBackend + ?Sized,
{
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        self.as_ref().get_kv(key).await
    }

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        self.as_ref().put_kv(key, value).await
    }

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError> {
        self.as_ref().get_action_result(digest).await
    }

    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError> {
        self.as_ref().put_action_result(digest, action_result).await
    }

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        self.as_ref().build_digest_from_hash_if_present(hash).await
    }

    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        self.as_ref().cas_filter_for_missing(digests).await
    }

    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        self.as_ref().cas_exists(digest).await
    }

    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError> {
        self.as_ref().cas_insert(digest, data).await
    }

    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataReturnTpe>, StorageBackendError> {
        self.as_ref().cas_get_data(digest).await
    }
}

#[async_trait::async_trait]
impl<T> StorageBackend for &T
where
    T: StorageBackend + ?Sized,
{
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        (*self).get_kv(key).await
    }

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        (*self).put_kv(key, value).await
    }

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError> {
        (*self).get_action_result(digest).await
    }

    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError> {
        (*self).put_action_result(digest, action_result).await
    }

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        (*self).build_digest_from_hash_if_present(hash).await
    }

    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        (*self).cas_filter_for_missing(digests).await
    }

    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        (*self).cas_exists(digest).await
    }

    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError> {
        (*self).cas_insert(digest, data).await
    }

    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataReturnTpe>, StorageBackendError> {
        (*self).cas_get_data(digest).await
    }
}
