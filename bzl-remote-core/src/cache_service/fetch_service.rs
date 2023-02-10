use crate::storage_backend::{BackendIOHelpers, StorageBackendError};
use crate::{hash::sha256_value::Sha256Value, storage_backend::StorageBackend};
use bazelfe_protos::build::bazel::remote::{
    asset::v1::{
        fetch_server, FetchBlobRequest, FetchBlobResponse, FetchDirectoryRequest,
        FetchDirectoryResponse,
    },
    execution::v2::Digest,
};
use tonic::Request;

#[derive(Debug)]
pub struct FetchServiceStruct<T> {
    storage_backend: T,
}

impl<T> FetchServiceStruct<T>
where
    T: StorageBackend,
{
    pub fn new(storage_backend: T) -> FetchServiceStruct<T> {
        FetchServiceStruct { storage_backend }
    }

    async fn fetch_one(&self, uris: Vec<String>) -> Result<(Digest, String), StorageBackendError> {
        let mut err = StorageBackendError::Unknown("empty uris vector".into());
        for uri in uris {
            match self
                .storage_backend
                .download_file_to_cas(uri.as_str())
                .await
            {
                Ok(d) => return Ok((d, uri.clone())),
                Err(e) => err = e,
            }
        }

        // if we got here we never hit the return Ok
        Err(err)
    }
}

fn extract_req_digest(fetch_blob_request: &FetchBlobRequest) -> Result<String, tonic::Status> {
    for dig in fetch_blob_request.qualifiers.iter() {
        if dig.name == "checksum.sri" {
            if let Some(sha_v) = dig.value.strip_prefix("sha256-") {
                let v = base64::decode(sha_v)
                    .map_err(|ex| tonic::Status::invalid_argument(format!("{:#?}", ex)))?;
                // The result is a binary sha256 value
                match Sha256Value::new_from_slice(&v) {
                    Err(e) => return Err(tonic::Status::invalid_argument(format!("{:#?}", e))),
                    Ok(o) => return Ok(o.to_string()),
                }
            }
        }
    }
    Err(tonic::Status::invalid_argument(
        "Request had no sha256 digest info included.",
    ))
}

#[tonic::async_trait]
impl<T> fetch_server::Fetch for FetchServiceStruct<T>
where
    T: StorageBackend + 'static,
{
    async fn fetch_blob(
        &self,
        req: Request<FetchBlobRequest>,
    ) -> Result<tonic::Response<FetchBlobResponse>, tonic::Status> {
        let fetch_blob_request = req.into_inner();

        let found_digest = extract_req_digest(&fetch_blob_request)?;

        let digest = self
            .storage_backend
            .build_digest_from_hash_if_present(&found_digest)
            .await?;

        let (digest, found_url) = match digest {
            None => match self.fetch_one(fetch_blob_request.uris).await {
                Ok((d, url)) => {
                    if d.hash != found_digest {
                        return Err(tonic::Status::internal(format!("Tried to fetch/download the hash {}, but ended up with a digest of {:?} which is incorrect", found_digest, digest)));
                    }
                    (d, Some(url))
                }
                Err(ex) => {
                    return Err(tonic::Status::internal(format!(
                        "Unable to fetch with error {:#?}",
                        ex
                    )))
                }
            },
            Some(d) => (d, None),
        };

        tracing::info!(
            "Asked to a fetch to find remote resource for hash: {digest:?}, if downloaded url: {url:?}",
            digest = digest.hash,
            url = found_url
        );
        Ok(tonic::Response::new(FetchBlobResponse {
            blob_digest: Some(digest),
            ..Default::default()
        }))
    }
    async fn fetch_directory(
        &self,
        _: Request<FetchDirectoryRequest>,
    ) -> std::result::Result<tonic::Response<FetchDirectoryResponse>, tonic::Status> {
        todo!("fetch directory not yet implemented")
    }
}
