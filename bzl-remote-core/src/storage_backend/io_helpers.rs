use bazelfe_protos::build::bazel::remote::execution::v2::Digest;
use futures::StreamExt;
use http::header;
use sha2::Digest as Sha2Digest;
use tempfile::NamedTempFile;
use tokio::io::AsyncWriteExt;
use hyper_util::client::legacy::Client;

use crate::hash::sha256_value::Sha256Value;

use super::{StorageBackend, StorageBackendError, UploadType};

pub async fn store_body_in_cas<T: StorageBackend>(
    storage_backend: &T,
    body: &mut impl http_body::Body<Data = bytes::Bytes, Error = hyper::Error>,
) -> Result<Digest, StorageBackendError> {
    let tmp_file = NamedTempFile::new()?;
    let mut tokio_output = tokio::fs::File::create(tmp_file.path()).await?;
    let mut total_bytes = 0;
    let mut hasher = sha2::Sha256::new();
    while let Some(chunk) = body.next().await {
        let data = chunk.map_err(|err| {
            StorageBackendError::Unknown(format!("IO error occured downloading chunks {:#?}", err))
        })?;

        total_bytes += data.len();
        if !data.is_empty() {
            hasher.update(&data[..]);
        }
        tokio_output.write_all(&data[..]).await?;
    }
    tokio_output.flush().await?;
    drop(tokio_output);

    let sha = Sha256Value::new_from_slice(&hasher.finalize())?;

    let digest = Digest {
        hash: sha.to_string(),
        size_bytes: total_bytes as i64,
    };

    storage_backend
        .cas_insert(&digest, UploadType::OnDisk(tmp_file.path().to_path_buf()))
        .await?;

    Ok(digest)
}

#[async_trait::async_trait]
pub trait BackendIOHelpers: StorageBackend + Sized {
    async fn download_file_to_cas(&self, url: &str) -> Result<Digest, StorageBackendError>;
}

fn compute_redirect(old_url: &str, location_uri: &str) -> Result<hyper::Uri, StorageBackendError> {
    let old_uri = old_url.parse::<hyper::Uri>().map_err(|e| {
        StorageBackendError::Unknown(format!("Unable to parse url with error {:#?}", e))
    })?;
    let location_uri = location_uri.parse::<hyper::Uri>().map_err(|e| {
        StorageBackendError::Unknown(format!("Unable to parse url with error {:#?}", e))
    })?;

    let old_parts = old_uri.into_parts();
    let mut location_parts = location_uri.into_parts();

    if location_parts.authority.is_none() {
        location_parts.authority = old_parts.authority;
    }

    if location_parts.scheme.is_none() {
        location_parts.scheme = old_parts.scheme;
    }

    location_parts.try_into().map_err(|e| {
        StorageBackendError::Unknown(format!("Unable to convert parts back into uri {:#?}", e))
    })
}

const MAX_REDIRECT_DEPTH: usize = 5;

#[derive(Debug)]
enum RedirectOrValue {
    Redirect(String),
    Value(Digest),
}

async fn inner_download_file_to_cas<T: StorageBackend>(
    storage: &T,
    url: &str,
) -> Result<RedirectOrValue, StorageBackendError> {
    let uri = url.parse::<hyper::Uri>().unwrap();
    let https = hyper_tls::HttpsConnector::new();
    let client = Client::builder().build::<_, http_body::Body>(https);
    match client.get(uri).await {
        Ok(mut res) => {
            if res.status().is_redirection() {
                let new_url = res.headers().get(header::LOCATION);
                if let Some(header_v) = new_url {
                    let location_uri = header_v.to_str().map_err(|e| {
                        StorageBackendError::Unknown(format!(
                            "Location header invalid, unable to make it a string {:#?}",
                            e
                        ))
                    })?;
                    let new_uri = compute_redirect(url, location_uri)?;

                    let u = new_uri.to_string();
                    return Ok(RedirectOrValue::Redirect(u));
                } else {
                    return Err(StorageBackendError::Unknown(format!(
                        "Got a redirection code:, but had no Location header {:?}",
                        res.status()
                    )));
                }
            }
            if !res.status().is_success() {
                Err(StorageBackendError::Unknown(format!(
                    "couldn't fetch uri {}, had status {:?}",
                    url,
                    res.status()
                )))
            } else {
                let r = store_body_in_cas(storage, res.body_mut()).await?;
                Ok(RedirectOrValue::Value(r))
            }
        }
        Err(err) => Err(StorageBackendError::ErrorAndMessage(
            format!("couldn't fetch uri {}", url),
            Box::new(err),
        )),
    }
}
#[async_trait::async_trait]
impl<T: StorageBackend> BackendIOHelpers for T {
    async fn download_file_to_cas(&self, url: &str) -> Result<Digest, StorageBackendError> {
        let mut next_url: String = url.to_string();

        for _ in 0..MAX_REDIRECT_DEPTH {
            match inner_download_file_to_cas(self, next_url.as_str()).await? {
                RedirectOrValue::Value(v) => return Ok(v),
                RedirectOrValue::Redirect(url) => {
                    next_url = url;
                }
            }
        }

        return Err(StorageBackendError::Unknown(format!(
            "Have followed too many redirects, Original url: {:#?}, final url: {:#?}",
            url, next_url
        )));
    }
}
