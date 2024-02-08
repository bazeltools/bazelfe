use bazelfe_protos::{
    build::bazel::remote::execution::v2::{self as execution},
    bzl_remote::metadata_service::metadata_service_client::MetadataServiceClient,
    google::{
        self,
        bytestream::{byte_stream_client::ByteStreamClient, ReadRequest},
    },
};
use std::{path::Path, str::FromStr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tonic::{
    transport::{Channel, Endpoint},
    Request,
};

#[derive(Clone, Debug)]
pub struct CacheClient {
    content_addressable_store: ContentAddressableStore,
}

impl CacheClient {
    fn new(channel: Channel) -> CacheClient {
        CacheClient {
            content_addressable_store: ContentAddressableStore::new(channel.clone()),
        }
    }
    pub async fn connect<S: AsRef<str>>(
        connect_url: S,
    ) -> Result<CacheClient, Box<dyn std::error::Error + Send + Sync>> {
        let channel = Endpoint::from_str(connect_url.as_ref())?
            .http2_adaptive_window(true)
            .connect()
            .await?;

        Ok(CacheClient::new(channel))
    }

    pub fn cas(&self) -> &ContentAddressableStore {
        &self.content_addressable_store
    }
}

#[derive(Clone, Debug)]
pub struct ContentAddressableStore {
    bytestream_cli: ByteStreamClient<Channel>,
    metadata_cli: MetadataServiceClient<Channel>,
}

impl ContentAddressableStore {
    fn new(channel: Channel) -> ContentAddressableStore {
        ContentAddressableStore {
            bytestream_cli: ByteStreamClient::new(channel.clone()),
            metadata_cli: MetadataServiceClient::new(channel.clone()),
        }
    }

    pub async fn find_digest(
        &self,
        digest_hash: &str,
    ) -> Result<Option<execution::Digest>, Box<dyn std::error::Error + Send + Sync>> {
        let mut metadata_cli = self.metadata_cli.clone();
        let len_resp = metadata_cli
            .lookup_len(
                bazelfe_protos::bzl_remote::metadata_service::LookupLenRequest {
                    hash: digest_hash.to_string(),
                },
            )
            .await?
            .into_inner();

        if let Some(digest) = len_resp.digest {
            Ok(Some(digest))
        } else {
            Ok(None)
        }
    }

    pub async fn upload_file(
        &self,
        digest: &execution::Digest,
        path: &Path,
        send_buffer_size: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut cli = self.bytestream_cli.clone();

        let mut file = tokio::fs::File::open(path).await?;
        let len = digest.size_bytes as usize;

        let resource_url = format!(
            "uploads/936DA01F9ABD4d9d80C702AF85C822A8{}/blobs/{}/{}",
            rand::random::<usize>(),
            digest.hash,
            digest.size_bytes
        );
        let send_buffer_size = send_buffer_size as usize;

        let outbound = async_stream::stream! {
            let mut remaining = len;
            let mut write_offset = 0;


            loop {
                let next_buff_size = std::cmp::min(send_buffer_size, remaining) as usize;
                let mut buf = vec![0; next_buff_size];

                let mut bytes_read = 0;
                while bytes_read < next_buff_size {
                    match file.read(&mut buf).await {
                        Err(e) => {
                            tracing::error!("Attempted to read from file, but failed with error: {:?}",e);
                            break;
                        },
                        Ok(u) =>
                        {
                            bytes_read += u;
                            if u == 0 {
                                break;
                            }
                        }
                    };
                }

                remaining -= bytes_read;


                if buf.len() != bytes_read {
                    buf.truncate(bytes_read);
                }
                let buf = google::bytestream::WriteRequest {
                    data: buf,
                    finish_write: remaining == 0,
                    resource_name: if write_offset == 0 { resource_url.clone() } else { String::from("") },
                    write_offset: write_offset as i64
                };
                write_offset += bytes_read;

                yield buf;
                if remaining == 0 {
                    break;
                }
            }
        };

        cli.write(Request::new(outbound)).await?;
        Ok(())
    }

    pub async fn fetch_to_path(
        &self,
        digest: &execution::Digest,
        path: &Path,
    ) -> Result<Option<()>, Box<dyn std::error::Error + Send + Sync>> {
        let mut cli = self.bytestream_cli.clone();

        let mut output_file = tokio::fs::File::create(path).await?;

        let read_response = cli
            .read(Request::new(ReadRequest {
                resource_name: format!("blobs/{}/{}", digest.hash, digest.size_bytes),
                ..Default::default()
            }))
            .await;

        match read_response {
            Ok(read_response) => {
                let mut stream = read_response.into_inner();
                let mut written = 0;
                while let Some(resp) = stream.message().await? {
                    output_file.write_all(&resp.data).await?;
                    written += resp.data.len();
                }
                if written != digest.size_bytes as usize {
                    return Err(anyhow::anyhow!("Got wrong size result back").into());
                }
                Ok(Some(()))
            }
            Err(err) => {
                if err.code() == tonic::Code::NotFound {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    pub async fn fetch_to_bytes(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let mut cli = self.bytestream_cli.clone();

        let mut result: Vec<u8> = Vec::with_capacity(digest.size_bytes as usize);

        let read_response = cli
            .read(Request::new(ReadRequest {
                resource_name: format!("blobs/{}/{}", digest.hash, digest.size_bytes),
                ..Default::default()
            }))
            .await;

        match read_response {
            Ok(read_response) => {
                let mut stream = read_response.into_inner();
                while let Some(mut resp) = stream.message().await? {
                    result.append(&mut resp.data);
                }
                if result.len() != digest.size_bytes as usize {
                    return Err(anyhow::anyhow!("Got wrong size result back").into());
                }
                Ok(Some(result))
            }
            Err(err) => {
                if err.code() == tonic::Code::NotFound {
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }
}
