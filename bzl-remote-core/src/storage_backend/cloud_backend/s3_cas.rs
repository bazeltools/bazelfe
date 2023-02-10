use std::{path::Path, sync::Arc};

use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use crate::storage_backend::StorageBackendError;

use super::s3::S3;

#[derive(Debug)]
pub struct S3Cas {
    s3: Arc<S3>,
    cas_prefix: String,
}

impl S3Cas {
    pub async fn new<Sa: AsRef<str>>(
        s3: &Arc<S3>,
        cas_prefix: Sa,
    ) -> Result<S3Cas, StorageBackendError> {
        Ok(S3Cas {
            s3: Arc::clone(s3),
            cas_prefix: cas_prefix.as_ref().to_string(),
        })
    }

    fn as_s3_path(&self, digest: &execution::Digest) -> String {
        format!("{}/{}", self.cas_prefix, digest.hash)
    }

    #[allow(unused)]
    pub async fn exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        self.s3.exists(self.as_s3_path(digest)).await
    }

    pub async fn upload(
        &self,
        local_path: &Path,
        digest: &execution::Digest,
    ) -> Result<(), StorageBackendError> {
        self.s3.upload(local_path, self.as_s3_path(digest)).await
    }

    #[allow(unused)]
    pub async fn upload_bytes(
        &self,
        bytes: Vec<u8>,
        digest: &execution::Digest,
    ) -> Result<(), StorageBackendError> {
        self.s3.upload_bytes(bytes, self.as_s3_path(digest)).await
    }

    pub async fn download_bytes(
        &self,
        digest: &execution::Digest,
    ) -> Result<Vec<u8>, StorageBackendError> {
        self.s3.download_bytes(self.as_s3_path(digest)).await
    }

    pub async fn download(
        &self,
        local_path: &Path,
        digest: &execution::Digest,
    ) -> Result<(), StorageBackendError> {
        self.s3.download(local_path, self.as_s3_path(digest)).await
    }
}
