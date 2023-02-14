use bzl_remote_core::cache_service::action_cache_service::ActionCacheService;
use bzl_remote_core::cache_service::bytestream_service::ByteStreamService;
use bzl_remote_core::cache_service::capabilities_service::CapabilitiesService;
use bzl_remote_core::cache_service::content_addressable_storage_service::ContentAddressableStorageService;

use bazelfe_protos::build::bazel::remote::asset::v1::fetch_server::FetchServer;
use bazelfe_protos::build::bazel::remote::execution::v2::action_cache_server::ActionCacheServer;
use bazelfe_protos::build::bazel::remote::execution::v2::capabilities_server::CapabilitiesServer;
use bazelfe_protos::build::bazel::remote::execution::v2::content_addressable_storage_server::ContentAddressableStorageServer;
use bazelfe_protos::google::bytestream::byte_stream_server::ByteStreamServer;
use bzl_remote_core::cache_service::fetch_service::FetchServiceStruct;
use bzl_remote_core::cache_service::http_endpoint::HttpEndpoint;
use bzl_remote_core::cache_service::metadata_service::MetadataService;
use bzl_remote_core::server::{EitherBody, GrpcErrorTraceLayer};
use clap::Parser;
use futures::future::{self, Either, TryFutureExt};
use http::version::Version;

use hyper::{service::make_service_fn, Server};
use std::convert::Infallible;

use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tonic::transport::Server as TonicServer;
use tower::Service;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Parser, Debug)]
#[clap(name = "basic")]
struct ServerAppArgs {
    #[clap(long)]
    pub config_path: Option<PathBuf>,

    #[clap(long, env = "BIND_ADDRESS")]
    bind_address: Option<String>,

    #[clap(long)]
    logs_root: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = ServerAppArgs::parse();

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var(
            "RUST_LOG",
            "info,bzl_remote_core=info,cache_server=info,aws_config=warn",
        );
    }

    let (non_blocking, _guard) = if let Some(logs_root) = opt.logs_root.as_ref() {
        let logs_root = logs_root.clone();
        tracing_appender::non_blocking(tracing_appender::rolling::hourly(
            logs_root,
            "cache_server.log",
        ))
    } else {
        tracing_appender::non_blocking(std::io::stderr())
    };

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(non_blocking)
        .init();

    let mut config: bzl_remote_core::config::Config = bzl_remote_core::config::load_config_file(
        &opt.config_path.as_deref(),
        "bazel_cache_server.conf",
    )?;

    if let Some(addr) = opt.bind_address {
        let addr: std::net::SocketAddr = addr.parse()?;
        config.bind_address = Some(addr);
    }

    let bind_address = config
        .bind_address
        .unwrap_or_else(|| "127.0.0.1:10000".parse().unwrap());

    tracing::info!("Ready to serve on {}", bind_address);

    let storage_backend =
        match bzl_remote_core::cache_service::storage_backend_from_config(&config).await {
            Ok(r) => r,
            Err(ex) => {
                tracing::error!("Fatal error attemping to startup: {:#?}", ex);
                return Err(ex);
            }
        };

    Server::bind(&bind_address)
        .serve(make_service_fn(|_| {

            let action_cache_service = ActionCacheService::new(storage_backend.clone());
            let contentaddressablestorage_service =
                ContentAddressableStorageService::new(storage_backend.clone());
            let bytestream_service =
                ByteStreamService::new(storage_backend.clone(), config.send_buffer_size);

            let fetch_srv = FetchServiceStruct::new(storage_backend.clone());
            let metadata_service = MetadataService::new(storage_backend.clone());

            let layer = tower::ServiceBuilder::new()
                // Apply our own middleware
                .layer(GrpcErrorTraceLayer::default())
                .into_inner();

            let capabilites_service = CapabilitiesService::new();
            let mut grpc_server = TonicServer::builder()
                .max_concurrent_streams(1000)
                .tcp_nodelay(true)
                .initial_stream_window_size(16384*2)
                .trace_fn(|r| {
                    let ip_info = r.extensions()
                    .get::<tonic::transport::server::TcpConnectInfo>()
                    .and_then(|i| i.remote_addr())
                    .or_else(|| {
                        r.extensions()
                            .get::<tonic::transport::server::TlsConnectInfo<tonic::transport::server::TcpConnectInfo>>()
                            .and_then(|i| i.get_ref().remote_addr())
                    });


                    let remote_ip = ip_info.map(|e| e.ip().to_string());
                    let remote_str = remote_ip.as_ref();
                    tracing::info_span!("bzl-cache", remote_client=?remote_str)
                })
                .max_frame_size(6194304)
                .layer(layer)
                .add_service(ActionCacheServer::new(action_cache_service))
                .add_service(bazelfe_protos::bzl_remote::metadata_service::metadata_service_server::MetadataServiceServer::new(metadata_service))
                .add_service(CapabilitiesServer::new(capabilites_service))
                .add_service(ContentAddressableStorageServer::new(
                    contentaddressablestorage_service,
                ))
                .add_service(ByteStreamServer::new(bytestream_service))
                .add_service(FetchServer::new(fetch_srv))
                .into_service();

                let http_endpoint = Arc::new(HttpEndpoint::new(storage_backend.clone()));
                future::ok::<_, Infallible>(tower::service_fn(

                    move |req: hyper::Request<hyper::Body>| {
                        let http_endpoint = http_endpoint.clone();
                        match req.version() {
                        Version::HTTP_11 | Version::HTTP_10 =>
                            Either::Left(async move {
                            http_endpoint.dispatch(req)
                                .map_ok(|res| res.map(EitherBody::Left))
                                .map_err(BoxError::from)
                                .await
                            }

                        )
                    ,
                        Version::HTTP_2 => Either::Right(
                            grpc_server
                                .call(req)
                                .map_ok(|res| res.map(EitherBody::Right))
                                .map_err(BoxError::from),
                        ),
                        _ => unimplemented!(),
                    }},
                ))
    }))
    .await?;

    Ok(())
}
