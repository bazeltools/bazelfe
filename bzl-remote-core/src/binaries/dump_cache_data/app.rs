use bzl_remote_core::{cache_client::CacheClient, hash::sha256_value::Sha256Value};
use clap::{Parser, Subcommand};

use tokio::io::AsyncReadExt;

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use anyhow::{Context, Result};
use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use anyhow::anyhow;
use prost::Message;

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
enum ProtobufType {
    ActionResult,
    Action,
    Command,
}

#[derive(Clone)]
enum DecodedProtobuf {
    ActionResult(execution::ActionResult),
    Command(execution::Command),
    Action(execution::Action),
}
impl std::fmt::Debug for DecodedProtobuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Command(a) => a.fmt(f),
            Self::ActionResult(a) => a.fmt(f),
            Self::Action(a) => a.fmt(f),
        }
    }
}

#[derive(Clone, Debug)]
enum DataSource {
    BazelDiskCache(BazelDiskCache),
    GrpcServer(GrpcServer),
}

#[derive(Clone, Debug)]
struct BazelDiskCache {
    path: PathBuf,
}

#[derive(Clone, Debug)]
struct GrpcServer {
    cache_client: bzl_remote_core::cache_client::CacheClient,
}
#[async_trait::async_trait]
impl FetchData for DataSource {
    async fn upload_to_cache(
        &self,
        path: &Path,
        chunk_size: u64,
    ) -> Result<execution::Digest, anyhow::Error> {
        match self {
            DataSource::BazelDiskCache(s) => s.upload_to_cache(path, chunk_size).await,
            DataSource::GrpcServer(s) => s.upload_to_cache(path, chunk_size).await,
        }
    }

    async fn maybe_fetch_to_path(
        &self,
        digest: &str,
        path: &Path,
    ) -> Result<Option<()>, anyhow::Error> {
        match self {
            DataSource::BazelDiskCache(s) => s.maybe_fetch_to_path(digest, path).await,
            DataSource::GrpcServer(s) => s.maybe_fetch_to_path(digest, path).await,
        }
    }

    async fn maybe_fetch_bytes(&self, digest: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        match self {
            DataSource::BazelDiskCache(s) => s.maybe_fetch_bytes(digest).await,
            DataSource::GrpcServer(s) => s.maybe_fetch_bytes(digest).await,
        }
    }
}

#[async_trait::async_trait]
impl FetchData for GrpcServer {
    async fn upload_to_cache(
        &self,
        path: &Path,
        chunk_size: u64,
    ) -> Result<execution::Digest, anyhow::Error> {
        let metadata = std::fs::metadata(path)?;
        let sha_v = Sha256Value::from_path(path).await?;
        let digest = execution::Digest {
            size_bytes: metadata.len() as i64,
            hash: sha_v.to_string(),
        };

        self.cache_client
            .cas()
            .upload_file(&digest, path, chunk_size)
            .await
            .map_err(|e| anyhow!(e))?;
        Ok(digest)
    }

    async fn maybe_fetch_to_path(
        &self,
        digest_hash: &str,
        dest_path: &Path,
    ) -> Result<Option<()>, anyhow::Error> {
        let digest = if let Some(digest) = self
            .cache_client
            .cas()
            .find_digest(digest_hash)
            .await
            .map_err(|e| anyhow!(e))?
        {
            digest
        } else {
            return Ok(None);
        };

        Ok(self
            .cache_client
            .cas()
            .fetch_to_path(&digest, dest_path)
            .await
            .map_err(|e| anyhow!(e))?)
    }
    async fn maybe_fetch_bytes(&self, digest_hash: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let digest = if let Some(digest) = self
            .cache_client
            .cas()
            .find_digest(digest_hash)
            .await
            .map_err(|e| anyhow!(e))?
        {
            digest
        } else {
            return Ok(None);
        };

        Ok(self
            .cache_client
            .cas()
            .fetch_to_bytes(&digest)
            .await
            .map_err(|e| anyhow!(e))?)
    }
}

#[async_trait::async_trait]
impl FetchData for BazelDiskCache {
    async fn upload_to_cache(
        &self,
        _path: &Path,
        _chunk_size: u64,
    ) -> Result<execution::Digest, anyhow::Error> {
        todo!()
    }

    async fn maybe_fetch_to_path(
        &self,
        digest: &str,
        dest_path: &Path,
    ) -> Result<Option<()>, anyhow::Error> {
        let path = PathBuf::from(&self.path)
            .join("cas")
            .join(&digest[0..2])
            .join(digest);
        if !path.exists() {
            return Ok(None);
        }
        tokio::fs::copy(&path, dest_path).await?;

        Ok(Some(()))
    }
    async fn maybe_fetch_bytes(&self, digest: &str) -> Result<Option<Vec<u8>>, anyhow::Error> {
        let path = PathBuf::from(&self.path)
            .join("cas")
            .join(&digest[0..2])
            .join(digest);
        if !path.exists() {
            return Ok(None);
        }

        let mut file = tokio::fs::File::open(&path).await?;

        let mut data = Vec::default();
        file.read_to_end(&mut data).await?;

        Ok(Some(data))
    }
}

#[async_trait::async_trait]
trait FetchData {
    async fn maybe_fetch_bytes(&self, digest: &str) -> Result<Option<Vec<u8>>, anyhow::Error>;
    async fn maybe_fetch_to_path(
        &self,
        digest: &str,
        path: &Path,
    ) -> Result<Option<()>, anyhow::Error>;

    async fn upload_to_cache(
        &self,
        path: &Path,
        chunk_size: u64,
    ) -> Result<execution::Digest, anyhow::Error>;
}

enum PackerV {
    Value(Box<dyn std::fmt::Debug + Send + Sync + 'static>),
    Layer(String, HashMap<String, PackerV>),
}
impl std::fmt::Debug for PackerV {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(a) => a.fmt(f),
            Self::Layer(nme, v) => {
                let mut r = f.debug_struct(&nme);
                for (k, v) in v.iter() {
                    r.field(k, v);
                }
                r.finish()
            }
        }
    }
}

#[async_trait::async_trait]
trait Unroller {
    async fn unroll(&self, fd: Arc<dyn FetchData + Send + Sync>) -> Result<PackerV, anyhow::Error>;
}

#[async_trait::async_trait]
impl Unroller for execution::Directory {
    async fn unroll(
        &self,
        _fd: Arc<dyn FetchData + Send + Sync>,
    ) -> Result<PackerV, anyhow::Error> {
        Ok(PackerV::Value(Box::new(String::from("tmp"))))
    }
}

#[async_trait::async_trait]
impl Unroller for execution::Command {
    async fn unroll(
        &self,
        _fd: Arc<dyn FetchData + Send + Sync>,
    ) -> Result<PackerV, anyhow::Error> {
        let mut cur_v = HashMap::default();

        cur_v.insert(
            String::from("arguments"),
            PackerV::Value(Box::new(self.arguments.clone())),
        );

        let mut env_vars: HashMap<String, String> = HashMap::default();
        for env_var in &self.environment_variables {
            env_vars.insert(env_var.name.clone(), env_var.value.clone());
        }
        cur_v.insert(
            String::from("environment_variables"),
            PackerV::Value(Box::new(env_vars)),
        );
        cur_v.insert(
            String::from("output_files"),
            PackerV::Value(Box::new(self.output_files.clone())),
        );
        cur_v.insert(
            String::from("output_directories"),
            PackerV::Value(Box::new(self.output_directories.clone())),
        );
        cur_v.insert(
            String::from("output_paths"),
            PackerV::Value(Box::new(self.output_paths.clone())),
        );
        if let Some(platform) = self.platform.as_ref() {
            cur_v.insert(
                String::from("platform"),
                PackerV::Value(Box::new(platform.clone())),
            );
        }

        cur_v.insert(
            String::from("working_directory"),
            PackerV::Value(Box::new(self.working_directory.clone())),
        );
        cur_v.insert(
            String::from("output_node_properties"),
            PackerV::Value(Box::new(self.output_node_properties.clone())),
        );
        Ok(PackerV::Layer(String::from("Command"), cur_v))
    }
}

#[async_trait::async_trait]
impl Unroller for execution::Action {
    async fn unroll(&self, fd: Arc<dyn FetchData + Send + Sync>) -> Result<PackerV, anyhow::Error> {
        let mut cur_v = HashMap::default();

        if let Some(digest) = &self.command_digest.as_ref().map(|e| &e.hash) {
            if let Some(data) = &fd.maybe_fetch_bytes(digest).await? {
                let s = execution::Command::decode(&data[..])?;
                cur_v.insert(String::from("command"), s.unroll(Arc::clone(&fd)).await?);
            } else {
                cur_v.insert(
                    String::from("command"),
                    PackerV::Value(Box::new(format!(
                        "Digest {} missing, looking for Command",
                        digest
                    ))),
                );
            }
        }

        if let Some(digest) = &self.input_root_digest.as_ref().map(|e| &e.hash) {
            if let Some(data) = &fd.maybe_fetch_bytes(digest).await? {
                let s = execution::Directory::decode(&data[..])?;
                cur_v.insert(
                    String::from("input_root_digest"),
                    s.unroll(Arc::clone(&fd)).await?,
                );
            } else {
                cur_v.insert(
                    String::from("input_root_digest"),
                    PackerV::Value(Box::new(format!("Missing: {}", digest))),
                );
            }
        }

        cur_v.insert(
            String::from("do_not_cache"),
            PackerV::Value(Box::new(self.do_not_cache)),
        );

        Ok(PackerV::Layer(String::from("Action"), cur_v))
    }
}

#[derive(clap::Args, Debug)]
struct DecodeProtoArgs {
    #[clap(long)]
    protobuf_type: ProtobufType,

    #[clap(long)]
    unroll: bool,

    #[clap(long)]
    digest: String,
}

#[derive(clap::Args, Debug)]
struct DownloadArgs {
    #[clap(long)]
    digest: String,

    #[clap(long)]
    local_path: PathBuf,
}

#[derive(clap::Args, Debug)]
struct UploadArgs {
    #[clap(long)]
    local_path: PathBuf,

    #[clap(long, default_value_t = 4194304)]
    chunk_size: u64,
}

#[derive(Subcommand, Debug)]
enum Commands {
    DecodeProto(DecodeProtoArgs),
    Download(DownloadArgs),
    Upload(UploadArgs),
}

#[derive(Parser, Debug)]
#[clap(name = "basic")]
#[clap(group(
    clap::ArgGroup::new("datasrc")
        .required(true)
        .multiple(false)
        .args(&["bazel_disk_cache_path", "remote_cache_grpc"]),
))]
struct AppArgs {
    #[clap(long, group = "datasrc")]
    bazel_disk_cache_path: Option<PathBuf>,

    #[clap(long, group = "datasrc")]
    remote_cache_grpc: Option<String>,

    #[clap(subcommand)]
    command: Commands,
}

async fn upload_to_cache(args: &UploadArgs, data_source: DataSource) -> Result<(), anyhow::Error> {
    let source_path = &args.local_path;
    if !source_path.exists() {
        return Err(anyhow!(
            "Path {:?} doesn't exist, cannot upload it",
            source_path
        ));
    }

    let start_time = Instant::now();
    let digest = data_source
        .upload_to_cache(source_path.as_path(), args.chunk_size)
        .await?;

    println!(
        "Upload of {:?}, resulting in digest: {:?} complete in {:?}",
        source_path,
        digest,
        start_time.elapsed()
    );

    Ok(())
}

async fn download_from_cache(
    args: &DownloadArgs,
    data_source: DataSource,
) -> Result<(), anyhow::Error> {
    let target_path = &args.local_path;
    if let Some(parent_path) = target_path.parent() {
        std::fs::create_dir_all(parent_path)?
    }

    let start_time = Instant::now();
    if data_source
        .maybe_fetch_to_path(&args.digest, target_path.as_path())
        .await?
        .is_none()
    {
        return Err(anyhow!("Unable to find requested digest: {}", &args.digest));
    };

    println!("Download complete in {:?}", start_time.elapsed());

    Ok(())
}

async fn decode_proto(
    args: &DecodeProtoArgs,
    data_source: DataSource,
) -> Result<(), anyhow::Error> {
    let data = match data_source.maybe_fetch_bytes(&args.digest).await? {
        None => {
            return Err(anyhow!("Path constructed for digest: {}", &args.digest));
        }
        Some(d) => d,
    };

    let decoded_proto = match args.protobuf_type {
        ProtobufType::ActionResult => {
            DecodedProtobuf::ActionResult(execution::ActionResult::decode(&data[..])?)
        }
        ProtobufType::Action => DecodedProtobuf::Action(execution::Action::decode(&data[..])?),
        ProtobufType::Command => DecodedProtobuf::Command(execution::Command::decode(&data[..])?),
    };

    if !args.unroll {
        println!("{:#?}", decoded_proto);
    } else {
        let fd: Arc<dyn FetchData + Send + Sync> = Arc::new(data_source);
        let packer_v = match decoded_proto {
            DecodedProtobuf::ActionResult(_e) => todo!(),
            DecodedProtobuf::Command(_) => todo!(),
            DecodedProtobuf::Action(e) => e.unroll(Arc::clone(&fd)).await?,
        };

        println!("{:#?}", packer_v);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let opt = AppArgs::parse();

    if ::std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info,bzl_remote_core=info,dump_cache_data=info");
    }

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stderr());
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .init();

    let data_source = match (&opt.bazel_disk_cache_path, &opt.remote_cache_grpc) {
        (None, Some(grpc)) => {
            let cache_client = CacheClient::connect(grpc)
                .await
                .map_err(|e| anyhow!(e))
                .with_context(|| "Connecting cache client")?;
            DataSource::GrpcServer(GrpcServer { cache_client })
        }
        (Some(path), None) => DataSource::BazelDiskCache(BazelDiskCache { path: path.clone() }),
        _ => unreachable!(),
    };

    match opt.command {
        Commands::DecodeProto(d) => decode_proto(&d, data_source).await,
        Commands::Download(d) => download_from_cache(&d, data_source).await,
        Commands::Upload(d) => upload_to_cache(&d, data_source).await,
    }
}
