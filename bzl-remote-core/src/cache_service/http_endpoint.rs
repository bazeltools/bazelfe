use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use bazelfe_protos::bzl_remote::bazelfe_index::BazelFeIndexLookupKey;
use bazelfe_protos::bzl_remote::bazelfe_kv::key_val_key;
use bazelfe_protos::bzl_remote::bazelfe_kv::KeyValKey;

use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
use futures::StreamExt;
use http::Method;

use http::Uri;
use hyper::{Body, Request, Response, StatusCode};
use prost::Message;
use sha2::Digest;
use sysinfo::Disks;
// Import the multer types.
use tempfile::NamedTempFile;

use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

use crate::hash::sha256_value::Sha256Value;
use crate::storage_backend::BackendIOHelpers;
use crate::storage_backend::{StorageBackend, StorageBackendError, UploadType};
static NOTFOUND: &[u8] = b"Not Found";

fn internal_server_error(message: String) -> Response<Body> {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body(message.into())
        .unwrap()
}

use thiserror::Error;

#[derive(Error, Debug)]
pub enum HttpEndpointError {
    #[error("Unknown datastore error: {0}")]
    Unknown(String),

    #[error("Unknown IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    InternalError(Box<dyn std::error::Error + Send + Sync>),

    #[error("Backend storage error: {0}")]
    StorageError(#[from] StorageBackendError),

    #[error("Unknown Error: {0} {1}")]
    ErrorAndMessage(String, Box<dyn std::error::Error + Send + Sync>),

    #[error("Url {0} got args: {1:?}")]
    MissingArgForMethod(String, Vec<String>),

    #[error("Bad Data error: {0}")]
    BadData(String),

    #[error("External IO failure talking to remote host: {0} with : {1}")]
    ExternalIOFailure(String, String),

    #[error("Seen Hyper error")]
    HyperError(#[from] hyper::Error),
}

#[derive(Debug)]
struct HealthStatus {
    last_update: Instant,
    last_status_code: StatusCode,
    last_description: String,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self {
            last_update: Instant::now(),
            last_status_code: StatusCode::OK,
            last_description: String::from("HEALTHY"),
        }
    }
}
impl HealthStatus {
    pub fn update(&mut self) {
        let now = Instant::now();
        if now.duration_since(self.last_update) > std::time::Duration::from_secs(30) {
            self.last_update = now;
            self.last_description = String::from("OK -- unable to find disk space");
            self.last_status_code = StatusCode::OK;
            let mut disks = Disks::new_with_refreshed_list();
            for disk in disks.list_mut() {
              //(&mut self.system).disks_mut() {
                if disk.mount_point() == PathBuf::from("/") {
                    disk.refresh();
                    let available_space = disk.available_space() as f64 / disk.total_space() as f64;
                    self.last_description = format!(
                        "Disk space available ({0:.2}%): {1} bytes total space: {2} bytes",
                        available_space * 100_f64,
                        disk.available_space(),
                        disk.total_space()
                    );
                    if available_space < 0.1 {
                        self.last_status_code = StatusCode::INSUFFICIENT_STORAGE;
                    }
                }
            }
        }
    }
}
#[derive(Debug)]
pub struct HttpEndpoint<T>
where
    T: StorageBackend + 'static,
{
    storage_backend: T,
    health_status: Arc<Mutex<HealthStatus>>,
}

async fn parse_cas_get(uri: &Uri) -> Result<execution::Digest, HttpEndpointError> {
    let segments: Vec<String> = uri
        .path()
        .split('/')
        .skip(1)
        .take(3)
        .map(|e| e.to_string())
        .collect();

    if segments.len() != 3 || segments[0] != "cas" {
        return Err(HttpEndpointError::MissingArgForMethod(
            String::from("GET /cas/<digest_sha>/<digest_len>"),
            segments,
        ));
    }

    Ok(execution::Digest {
        hash: segments[1].to_string(),
        size_bytes: segments[2].parse::<i64>().map_err(|_| {
            HttpEndpointError::MissingArgForMethod(
                String::from("GET /cas/<digest_sha>/<digest_len>, len did not parse"),
                segments,
            )
        })?,
    })
}

fn parse_bazel_fe_key(uri: &Uri) -> Result<BazelFeIndexLookupKey, HttpEndpointError> {
    let segments: Vec<String> = uri
        .path()
        .split('/')
        .skip(1)
        .take(4)
        .map(|e| e.to_string())
        .collect();

    if segments.len() != 4 || segments[0] != "bazelfe_index" {
        return Err(HttpEndpointError::MissingArgForMethod(
            String::from("PUT /bazelfe_index/<project>/<repository>/<commit_sha>"),
            segments,
        ));
    }
    let key = BazelFeIndexLookupKey {
        project: segments[1].to_string(),
        repo: segments[2].to_string(),
        commit_sha: segments[3].to_string(),
    };
    Ok(key)
}

#[derive(Debug, PartialEq, Eq)]
enum MirrorRequestUpstream {
    Github,
}

impl MirrorRequestUpstream {
    pub fn to_uri_root(&self) -> &str {
        match self {
            MirrorRequestUpstream::Github => "https://github.com",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
struct MirrorRequest {
    upstream: MirrorRequestUpstream,
    remaining_path: String,
    digest_hash: String,
}
impl MirrorRequest {
    pub(crate) fn uri(&self) -> String {
        format!("{}/{}", self.upstream.to_uri_root(), self.remaining_path)
    }
}

fn parse_mirror_request(uri: &Uri) -> Result<MirrorRequest, HttpEndpointError> {
    let segments: Vec<String> = uri
        .path()
        .split('/')
        .skip(1)
        .map(|e| e.to_string())
        .collect();

    if segments.len() < 4 || segments[0] != "upstream_mirror" {
        return Err(HttpEndpointError::MissingArgForMethod(
            String::from("PUT /upstream_mirror/<sha256>/<supported_upstream>/<remaining_url_path>"),
            segments,
        ));
    }
    let digest_hash = segments[1].clone();

    let m = match segments[2].as_str() {
        "github.com" => MirrorRequestUpstream::Github,
        _ => {
            return Err(HttpEndpointError::MissingArgForMethod(
                format!(
                    "upstream {} not in supported upstreams: github.com",
                    segments[2]
                ),
                segments,
            ))
        }
    };

    let key = MirrorRequest {
        upstream: m,
        digest_hash,
        remaining_path: segments
            .iter()
            .skip(3)
            .map(|e| e.as_str())
            .collect::<Vec<&str>>()
            .join("/"),
    };
    Ok(key)
}

impl<T: StorageBackend + 'static> HttpEndpoint<T> {
    pub fn new(storage_backend: T) -> HttpEndpoint<T> {
        HttpEndpoint {
            storage_backend,
            health_status: Default::default(),
        }
    }
    pub async fn dispatch(&self, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
        match self.inner_dispatch(req).await {
            Ok(r) => Ok(r),
            Err(err) => {
                tracing::warn!("Ran into error via dispatch of {:#?}", err);
                match err {
                    HttpEndpointError::HyperError(h) => Err(h),
                    HttpEndpointError::MissingArgForMethod(_, _) => Ok(Response::builder()
                        .status(StatusCode::NOT_ACCEPTABLE)
                        .body(format!("{:?}", err).into())
                        .unwrap()),
                    _ => Ok(internal_server_error(format!("{:?}", err))),
                }
            }
        }
    }

    async fn inner_dispatch(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, HttpEndpointError> {
        let first_segment = req.uri().path().split('/').nth(1).unwrap_or("");
        match (req.method(), first_segment) {
            (&Method::GET, "healthcheck") => {
                let health_status = Arc::clone(&self.health_status);
                tokio::spawn(async move {
                    let mut health_status = health_status.lock().await;
                    health_status.update();

                    Response::builder()
                        .status(health_status.last_status_code)
                        .body(health_status.last_description.clone().into())
                        .unwrap()
                })
                .await
                .map_err(|e| HttpEndpointError::Unknown(format!("Failure to join : {:#?}", e)))
            }
            (&Method::GET, "upstream_mirror") => self.process_upstream_mirror(req).await,
            (&Method::GET, "bazelfe_index") => {
                // Test what happens when file cannot be be found
                // simple_file_send("this_file_should_not_exist.html").await
                self.send_bazelfe_index(req).await
            }
            (&Method::PUT, "bazelfe_index") => {
                // Test what happens when file cannot be be found
                // simple_file_send("this_file_should_not_exist.html").await
                self.recv_bazelfe_index(req).await
            }

            (&Method::GET, "cas") => {
                let digest = parse_cas_get(req.uri()).await?;
                self.send_cas(&digest).await
            }
            (&Method::PUT, "cas") => self.recv_cas(req).await,
            _ => Ok(self.not_found()),
        }
    }

    async fn body_into_cas(
        &self,
        req: &mut Request<Body>,
    ) -> Result<execution::Digest, HttpEndpointError> {
        let tmp_file = NamedTempFile::new()?;
        let mut tokio_output = tokio::fs::File::create(tmp_file.path()).await.unwrap();
        let body = req.body_mut();
        let mut total_bytes = 0;
        let mut hasher = sha2::Sha256::new();
        while let Some(chunk) = body.next().await {
            let data = chunk?;

            total_bytes += data.len();
            if !data.is_empty() {
                hasher.update(&data[..]);
            }
            tokio_output.write_all(&data[..]).await?;
        }
        tokio_output.flush().await?;
        drop(tokio_output);

        let digest = match Sha256Value::new_from_slice(&hasher.finalize()) {
            Ok(sha) => execution::Digest {
                hash: sha.to_string(),
                size_bytes: total_bytes as i64,
            },
            Err(e) => {
                return Err(HttpEndpointError::ErrorAndMessage(
                    String::from("produced an invalid sha byte slice, shouldn't really happen"),
                    Box::new(e),
                ))
            }
        };

        self.storage_backend
            .cas_insert(&digest, UploadType::OnDisk(tmp_file.path().to_path_buf()))
            .await?;

        Ok(digest)
    }

    async fn recv_cas(&self, mut req: Request<Body>) -> Result<Response<Body>, HttpEndpointError> {
        let digest = self.body_into_cas(&mut req).await?;

        tracing::debug!("Receiving cas for produced digest: {:?}", digest);

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(format!("/cas/{}/{}", digest.hash, digest.size_bytes).into())
            .unwrap())
    }

    async fn recv_bazelfe_index(
        &self,
        mut req: Request<Body>,
    ) -> Result<Response<Body>, HttpEndpointError> {
        let key = parse_bazel_fe_key(req.uri())?;
        let digest = self.body_into_cas(&mut req).await?;

        tracing::debug!(
            "Receiving bazelfe index for configuration: {:?}, produced digest: {:?}",
            key,
            digest
        );

        let scoped_key = KeyValKey {
            keys: Some(key_val_key::Keys::BazelfeIndex(key)),
        };

        self.storage_backend
            .put_kv(&scoped_key.encode_to_vec(), &digest.encode_to_vec())
            .await?;

        Ok(Response::builder()
            .status(StatusCode::OK)
            .body(format!("{:#?}", digest).into())
            .unwrap())
    }

    async fn process_upstream_mirror(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, HttpEndpointError> {
        let mirror_request = parse_mirror_request(req.uri())?;
        tracing::debug!("Got mirror request for: {:?}", mirror_request);

        let digest = self
            .storage_backend
            .build_digest_from_hash_if_present(&mirror_request.digest_hash)
            .await?;

        let digest = match digest {
            None => match self
                .storage_backend
                .download_file_to_cas(mirror_request.uri().as_str())
                .await
            {
                Ok(d) => {
                    if d.hash != mirror_request.digest_hash {
                        return Err(HttpEndpointError::BadData(format!("Tried to fetch/download the hash {}, but ended up with a digest of {:?} which is incorrect", mirror_request.digest_hash, d), ));
                    }
                    d
                }
                Err(ex) => {
                    return Err(HttpEndpointError::ExternalIOFailure(
                        mirror_request.uri(),
                        format!("Unable to fetch with error {:#?}", ex),
                    ))
                }
            },
            Some(d) => d,
        };

        self.send_cas(&digest).await
    }

    async fn send_bazelfe_index(
        &self,
        req: Request<Body>,
    ) -> Result<Response<Body>, HttpEndpointError> {
        let key = parse_bazel_fe_key(req.uri())?;
        tracing::debug!("Requesting bazelfe index for configuration: {:?}", key);
        let scoped_key = KeyValKey {
            keys: Some(key_val_key::Keys::BazelfeIndex(key)),
        };

        let kv_resp = self
            .storage_backend
            .get_kv(&scoped_key.encode_to_vec())
            .await?;

        match kv_resp {
            None => {
                tracing::debug!("Didn't find scoped key in storage: {:?}", scoped_key);
                Ok(self.not_found())
            }
            Some(data) => {
                let digest = execution::Digest::decode(&data[..]).map_err(|e| {
                    tracing::error!("Failed to decode protobuf {:#?}", e);
                    HttpEndpointError::InternalError(Box::new(e))
                })?;
                self.send_cas(&digest).await
            }
        }
    }

    /// HTTP status code 404
    fn not_found(&self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(NOTFOUND.into())
            .unwrap()
    }

    async fn send_cas(
        &self,
        digest: &execution::Digest,
    ) -> Result<Response<Body>, HttpEndpointError> {
        let data = self.storage_backend.cas_get_data(digest).await?;
        let data = if let Some(d) = data {
            d
        } else {
            tracing::debug!("Didn't find digest in storage: {:?}", digest);
            return Ok(self.not_found());
        };

        // 32kb
        let buff_size = 32 * 1024;

        let stream = async_stream::try_stream! {
            let data = data;
            let buf = data.as_ref().as_ref();
            let mut remaining = buf.len();
            let mut offset = 0;
            while remaining > 0 {
                let next_segment_size = std::cmp::min(remaining, buff_size);
                let s = bytes::Bytes::copy_from_slice(&buf[offset..offset + next_segment_size]);
                offset += next_segment_size;
                remaining -= next_segment_size;

                yield s
            }
        };

        Ok(Response::new(hyper::Body::wrap_stream::<
            _,
            bytes::Bytes,
            Box<dyn std::error::Error + Send + Sync>,
        >(stream)))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_url() -> Result<(), Box<dyn std::error::Error>> {
        let uri = http::Uri::from_static("http://foo.com/bazelfe_index/myproj/myrepo/mysha");
        let parsed = parse_bazel_fe_key(&uri)?;

        let expected = BazelFeIndexLookupKey {
            project: String::from("myproj"),
            repo: String::from("myrepo"),
            commit_sha: String::from("mysha"),
        };

        assert_eq!(parsed, expected);

        Ok(())
    }

    #[test]
    fn test_parse_upstream_url() -> Result<(), Box<dyn std::error::Error>> {
        fn expect_bad_parse(url: &str) {
            let uri: Uri = url.parse().unwrap();
            assert!(
                parse_mirror_request(&uri).is_err(),
                "url {} should have failed to parse",
                url
            )
        }

        expect_bad_parse("http://foo.com/upstream_url/myproj/myrepo/mysha");
        expect_bad_parse("http://localhost:10000/upstream_mirreor/a3f37db1bf47603b45daebdd5012d1845adb2c21e6250e94c79014e5bc873b79/github.com/foo.tar.gz");
        expect_bad_parse("http://localhost:10000/upstream_mirror/github.com/foo.tar.gz");
        expect_bad_parse("http://localhost:10000/upstream_mirror/a3f37db1bf47603b45daebdd5012d1845adb2c21e6250e94c79014e5bc873b79/gdithub.com/foo.tar.gz");
        expect_bad_parse("http://localhost:10000/upstream_mirror/a3f37db1bf47603b45daebdd5012d1845adb2c21e6250e94c79014e5bc873b79/github.com");

        let uri = http::Uri::from_static("http://localhost:10000/upstream_mirror/a3f37db1bf47603b45daebdd5012d1845adb2c21e6250e94c79014e5bc873b79/github.com/foo.tar.gz");
        let parsed = parse_mirror_request(&uri)?;

        let expected = MirrorRequest {
            upstream: MirrorRequestUpstream::Github,
            remaining_path: String::from("foo.tar.gz"),
            digest_hash: String::from(
                "a3f37db1bf47603b45daebdd5012d1845adb2c21e6250e94c79014e5bc873b79",
            ),
        };

        assert_eq!(parsed, expected);

        Ok(())
    }
}
