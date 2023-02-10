use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use bazelfe_protos::*;
use futures::{Stream, TryStreamExt};

use regex::Regex;
use tracing::{Instrument, Span};

use std::pin::Pin;
use std::time::Instant;

use crate::storage_backend::{StorageBackend, UploadType};

use google::bytestream;
use lazy_static::lazy_static;

use tonic::{Request, Response};

#[derive(Debug)]
pub struct ByteStreamService<T>
where
    T: StorageBackend + 'static,
{
    storage_backend: T,
    send_buffer_size: usize,
}

#[derive(Debug)]
struct ConfiguredResource {
    pub _uuid: Option<String>,
    pub digest: execution::Digest,
}
lazy_static! {
    static ref READ_RESOURCE_REGEX: Regex =
        Regex::new("([^/]*/)?blobs/(?P<hash>\\w+)/(?P<size_bytes>\\d+).*").unwrap();


        // {instance_name}/uploads/{uuid}/blobs/{hash}/{size}
        static ref UPLOAD_RESOURCE_REGEX: Regex =
        Regex::new("([^/]*/)?uploads/(?P<uuid>[0-9A-Za-z-]+)/blobs/(?P<hash>\\w+)/(?P<size_bytes>\\d+).*").unwrap();

}

fn regex_resource_to_digest(
    resource_name: &str,
    regex: &Regex,
) -> Result<ConfiguredResource, tonic::Status> {
    if let Some(cap) = regex.captures(resource_name) {
        let hash = cap
            .name("hash")
            .ok_or_else(|| {
                tonic::Status::invalid_argument("Invalid resource name: Hash not present")
            })?
            .as_str();
        let raw_size_bytes = cap.name("size_bytes").ok_or_else(|| {
            tonic::Status::invalid_argument("Invalid resource name: size_bytes not present")
        })?;
        let size_bytes = raw_size_bytes
            .as_str()
            .parse::<i64>()
            .map_err(|_e| tonic::Status::invalid_argument("Unable to parse size"))?;
        let uuid = cap.name("uuid").map(|e| e.as_str().to_string());
        Ok(ConfiguredResource {
            _uuid: uuid,
            digest: execution::Digest {
                hash: hash.to_string(),
                size_bytes,
            },
        })
    } else {
        return Err(tonic::Status::invalid_argument(format!(
            "Invalid resource name: '{}'",
            resource_name
        )));
    }
}

impl<T> ByteStreamService<T>
where
    T: StorageBackend + 'static,
{
    pub fn new(storage_backend: T, send_buffer_size: usize) -> ByteStreamService<T> {
        ByteStreamService {
            storage_backend,
            send_buffer_size,
        }
    }
}

#[tonic::async_trait]
impl<T> bytestream::byte_stream_server::ByteStream for ByteStreamService<T>
where
    T: StorageBackend,
{
    type ReadStream = Pin<
        Box<
            dyn Stream<Item = Result<bytestream::ReadResponse, tonic::Status>>
                + Send
                + Sync
                + 'static,
        >,
    >;

    async fn read(
        &self,
        request: Request<bytestream::ReadRequest>,
    ) -> Result<tonic::Response<Self::ReadStream>, tonic::Status> {
        let start = Instant::now();
        let resource_name = request.get_ref().resource_name.to_string();

        let configured_resource = regex_resource_to_digest(&resource_name, &READ_RESOURCE_REGEX)?;

        let data_opt = self
            .storage_backend
            .cas_get_data(&configured_resource.digest)
            .await?;

        let found_data =
            data_opt.ok_or_else(|| tonic::Status::not_found("Unable to find requested file"))?;

        let (tx, rx) = flume::bounded(8);

        let send_buffer_size = self.send_buffer_size;
        tokio::spawn(
            async move {
                let slice_ref = found_data.as_ref().as_ref();
                let len = slice_ref.len();
                let mut remaining = len;
                let mut offset = 0;

                while remaining > 0 {
                    let next_buff_size = std::cmp::min(send_buffer_size, remaining) as usize;
                    let mut vec = Vec::with_capacity(next_buff_size);
                    vec.extend_from_slice(&slice_ref[offset..(next_buff_size + offset)]);
                    remaining -= next_buff_size;
                    offset += next_buff_size;

                    let buf = google::bytestream::ReadResponse { data: vec };
                    tx.send_async(Ok(buf)).await.unwrap();
                }
                let complete = Instant::now();
                // if slice_ref.len() > 4 * 1024 * 1024 {

                tracing::info!(
                    "Sending {resource_name} -- {size_mb} MB took {runtime:#?}",
                    resource_name = resource_name,
                    size_mb = slice_ref.len() as f64 / (1024_f64 * 1024_f64),
                    runtime = complete.duration_since(start)
                );
                // }
            }
            .instrument(Span::current()),
        );

        Ok(Response::new(Box::pin(rx.into_stream()) as Self::ReadStream))
    }

    async fn write(
        &self,
        request: Request<tonic::Streaming<bytestream::WriteRequest>>,
    ) -> Result<tonic::Response<bytestream::WriteResponse>, tonic::Status> {
        let mut stream = request.into_inner();
        let mut configured_resource: Option<ConfiguredResource> = None;
        let mut buff: Vec<u8> = vec![];

        let start_time = Instant::now();
        let mut waiting_timer: std::time::Duration = Default::default();
        let mut processing_timer: std::time::Duration = Default::default();
        loop {
            let waiting_instant = Instant::now();
            match stream.try_next().await {
                Ok(Some(element)) => {
                    waiting_timer += waiting_instant.elapsed();
                    let processing_instant = Instant::now();
                    if configured_resource.is_none() {
                        let c = regex_resource_to_digest(
                            &element.resource_name,
                            &UPLOAD_RESOURCE_REGEX,
                        )?;

                        if c.digest.size_bytes > 1024 * 1024 * 2 {
                            tracing::info!(
                                "Starting large upload of {}/{}",
                                &c.digest.hash,
                                c.digest.size_bytes
                            );
                        }
                        buff.reserve_exact(c.digest.size_bytes as usize);
                        configured_resource = Some(c);
                    }

                    buff.extend_from_slice(&element.data[..]);
                    processing_timer += processing_instant.elapsed()
                }
                Ok(None) => {
                    break;
                }
                other => {
                    tracing::error!("Received unexpected data on stream: {:#?}", other);
                    break;
                }
            }
        }

        let configured_resource = if let Some(r) = configured_resource {
            r
        } else {
            return Err(tonic::Status::failed_precondition(
                "Missing payload a configured resource",
            ));
        };

        if configured_resource.digest.size_bytes > 1024 * 1024 * 2 {
            let elapsed_seconds = (start_time.elapsed().as_millis() as f64) / 1000_f64;
            let size_f = configured_resource.digest.size_bytes as f64;
            let size_f = size_f / (1024_f64 * 1024_f64);

            tracing::info!(
                "Finished large upload of {:#?}, in {} seconds, throughput: {} MB/sec, waiting: {:?}, processing: {:?}",
                configured_resource,
                start_time.elapsed().as_secs(),
                size_f / elapsed_seconds,
                waiting_timer,
                processing_timer
            );
        }

        let committed_size = buff.len() as i64;
        if configured_resource.digest.size_bytes != committed_size {
            let error_msg = format!(
                "Tried to upload hash {} , size_bytes: {}, but received: {} bytes",
                configured_resource.digest.hash,
                configured_resource.digest.size_bytes,
                committed_size
            );
            tracing::warn!("{}", error_msg);
            return Err(tonic::Status::invalid_argument(error_msg));
        }

        self.storage_backend
            .cas_insert(&configured_resource.digest, UploadType::InMemory(buff))
            .await?;
        if configured_resource.digest.size_bytes > 1024 * 1024 * 2 {
            tracing::info!(
                "Inserted large upload of {}/{}, into the cache, total time is: {} secs",
                &configured_resource.digest.hash,
                configured_resource.digest.size_bytes,
                start_time.elapsed().as_secs()
            );
        }

        Ok(Response::new(bytestream::WriteResponse { committed_size }))
    }

    async fn query_write_status(
        &self,
        request: Request<bytestream::QueryWriteStatusRequest>,
    ) -> Result<tonic::Response<bytestream::QueryWriteStatusResponse>, tonic::Status> {
        let c =
            regex_resource_to_digest(&request.into_inner().resource_name, &UPLOAD_RESOURCE_REGEX)?;

        if self.storage_backend.cas_exists(&c.digest).await? {
            Ok(tonic::Response::new(bytestream::QueryWriteStatusResponse {
                committed_size: c.digest.size_bytes,
                complete: true,
            }))
        } else {
            Err(tonic::Status::unimplemented("Do not support"))
        }
    }
}
