use bazelfe_protos::bzl_remote::metadata_service;

use tonic::{Request, Response};

use crate::storage_backend::StorageBackend;

#[derive(Debug)]
pub struct MetadataService<T> {
    storage_backend: T,
}

impl<T> MetadataService<T>
where
    T: StorageBackend,
{
    pub fn new(storage_backend: T) -> MetadataService<T> {
        MetadataService { storage_backend }
    }
}

#[tonic::async_trait]
impl<T> metadata_service::metadata_service_server::MetadataService for MetadataService<T>
where
    T: StorageBackend + 'static,
{
    async fn lookup_len(
        &self,
        request: Request<metadata_service::LookupLenRequest>,
    ) -> Result<tonic::Response<metadata_service::LookupLenResponse>, tonic::Status> {
        let request = request.into_inner();

        let digest = self
            .storage_backend
            .build_digest_from_hash_if_present(&request.hash)
            .await?;

        Ok(Response::new(metadata_service::LookupLenResponse {
            digest,
        }))
    }
}
