use std::path::{Path, PathBuf};

use aws_sdk_s3::{config::Region, error::SdkError, primitives::ByteStream, Client};
use aws_smithy_http::error::Error as AwsError;
use futures::{StreamExt, TryStreamExt};

use crate::storage_backend::StorageBackendError;

#[derive(Debug)]
pub struct S3 {
    client: Client,
    bucket: String,
    prefix: PathBuf,
}

impl<E, R> From<SdkError<E, R>> for StorageBackendError
where
    E: std::error::Error + 'static,
    R: std::fmt::Debug,
{
    fn from(e: SdkError<E, R>) -> Self {
        StorageBackendError::Unknown(format!("{:?}", e))
    }
}

impl From<AwsError> for StorageBackendError {
    fn from(e: AwsError) -> Self {
        StorageBackendError::Unknown(format!("{:?}", e))
    }
}

impl S3 {
    pub async fn new<Sa: AsRef<str>, Sb: AsRef<str>, Sc: AsRef<str>>(
        region: Sa,
        bucket: Sb,
        prefix: Sc,
    ) -> Result<S3, Box<dyn std::error::Error>> {
        let region = Region::new(region.as_ref().to_string());
        let shared_config = aws_config::from_env().region(region).load().await;
        let client = Client::new(&shared_config);

        Ok(S3 {
            client,
            bucket: bucket.as_ref().to_string(),
            prefix: PathBuf::from(prefix.as_ref()),
        })
    }

    #[allow(unused)]
    pub async fn exists<S: AsRef<str>>(&self, path: S) -> Result<bool, StorageBackendError> {
        let request = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(self.prefix.join(path.as_ref()).to_string_lossy());

        match request.send().await {
            Ok(_) => Ok(true),
            Err(e) => {
                if let SdkError::ServiceError(inner_e) = &e {
                    if inner_e.err().is_not_found() {
                        Ok(false)
                    } else {
                        Err(e.into())
                    }
                } else {
                    Err(e.into())
                }
            }
        }
    }

    #[allow(unused)]
    pub async fn upload_bytes<S: AsRef<str>>(
        &self,
        bytes: Vec<u8>,
        path: S,
    ) -> Result<(), StorageBackendError> {
        let body: ByteStream = bytes::Bytes::from(bytes).into();

        let request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .body(body)
            .key(self.prefix.join(path.as_ref()).to_string_lossy());

        request.send().await?;
        Ok(())
    }

    pub async fn upload<S: AsRef<str>>(
        &self,
        local_path: &Path,
        path: S,
    ) -> Result<(), StorageBackendError> {
        let body = ByteStream::from_path(local_path).await?;

        let request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .body(body)
            .key(self.prefix.join(path.as_ref()).to_string_lossy());

        request.send().await?;
        Ok(())
    }

    pub async fn download<S: AsRef<str>>(
        &self,
        local_path: &Path,
        path: S,
    ) -> Result<(), StorageBackendError> {
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;

        let mut resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.prefix.join(path.as_ref()).to_string_lossy())
            .send()
            .await?;

        let mut file = File::create(local_path).await.map_err(|e| {
            StorageBackendError::ErrorAndMessage(
                format!(
                    "Error trying to make a file {}",
                    local_path.to_string_lossy()
                ),
                Box::new(e),
            )
        })?;

        let mut stream = std::mem::take(&mut resp.body).into_stream();
        while let Some(bytes) = stream.next().await {
            let bytes: bytes::Bytes = bytes?;
            file.write_all(&bytes).await.map_err(|e| {
                StorageBackendError::ErrorAndMessage(
                    format!(
                        "Error trying to write bytes to local file {}, bytes len: {}",
                        local_path.to_string_lossy(),
                        bytes.len()
                    ),
                    Box::new(e),
                )
            })?;
        }
        file.flush().await.map_err(|e| {
            StorageBackendError::ErrorAndMessage(
                format!("Failed to flush file {}", local_path.to_string_lossy()),
                Box::new(e),
            )
        })?;

        Ok(())
    }

    #[allow(unused)]
    pub async fn download_bytes<S: AsRef<str>>(
        &self,
        path: S,
    ) -> Result<Vec<u8>, StorageBackendError> {
        let mut resp = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(self.prefix.join(path.as_ref()).to_string_lossy())
            .send()
            .await?;

        let returned_data = std::mem::take(&mut resp.body).collect().await?;

        Ok(returned_data.into_bytes().to_vec())
    }
}
