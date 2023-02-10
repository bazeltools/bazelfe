use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use futures::Stream;

use std::pin::Pin;

use tonic::{Request, Response};

use crate::storage_backend::StorageBackend;

#[derive(Debug)]
pub struct ContentAddressableStorageService<T> {
    storage_backend: T,
}

impl<T> ContentAddressableStorageService<T>
where
    T: StorageBackend,
{
    pub fn new(storage_backend: T) -> ContentAddressableStorageService<T> {
        ContentAddressableStorageService { storage_backend }
    }
}

#[tonic::async_trait]
impl<T> execution::content_addressable_storage_server::ContentAddressableStorage
    for ContentAddressableStorageService<T>
where
    T: StorageBackend + 'static,
{
    type GetTreeStream = Pin<
        Box<
            dyn Stream<Item = Result<execution::GetTreeResponse, tonic::Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn find_missing_blobs(
        &self,
        request: Request<execution::FindMissingBlobsRequest>,
    ) -> Result<tonic::Response<execution::FindMissingBlobsResponse>, tonic::Status> {
        let request = request.into_inner();
        let mut blob_digests = request.blob_digests;

        self.storage_backend
            .cas_filter_for_missing(&mut blob_digests)
            .await?;

        Ok(Response::new(execution::FindMissingBlobsResponse {
            missing_blob_digests: blob_digests,
        }))
    }

    async fn batch_update_blobs(
        &self,
        request: Request<execution::BatchUpdateBlobsRequest>,
    ) -> Result<tonic::Response<execution::BatchUpdateBlobsResponse>, tonic::Status> {
        tracing::error!(
            "Unimplmented: batch_update_blobs - {:#?}",
            request.into_inner()
        );
        todo!()
    }

    async fn batch_read_blobs(
        &self,
        request: Request<execution::BatchReadBlobsRequest>,
    ) -> Result<tonic::Response<execution::BatchReadBlobsResponse>, tonic::Status> {
        tracing::error!(
            "Unimplmented: - batch_read_blobs {:#?}",
            request.into_inner()
        );
        todo!()
    }

    async fn get_tree(
        &self,
        request: Request<execution::GetTreeRequest>,
    ) -> Result<tonic::Response<Self::GetTreeStream>, tonic::Status> {
        tracing::error!("Unimplmented: get_tree {:#?}", request.into_inner());
        todo!()
    }
}
