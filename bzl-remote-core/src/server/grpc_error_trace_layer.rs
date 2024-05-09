use http_body::Body;
use hyper::Request;
use std::task::{Context, Poll};

use tonic::body::BoxBody;
use tower::{Layer, Service};

#[derive(Debug, Clone, Default)]
pub struct GrpcErrorTraceLayer;

impl<S> Layer<S> for GrpcErrorTraceLayer {
    type Service = GrpcErrorTraceService<S>;

    fn layer(&self, service: S) -> Self::Service {
        GrpcErrorTraceService { inner: service }
    }
}

#[derive(Debug, Clone)]
pub struct GrpcErrorTraceService<S> {
    inner: S,
}

impl<S, B> Service<Request<B>> for GrpcErrorTraceService<S>
where
    S: Service<Request<B>, Response = hyper::Response<BoxBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Body<Data = bytes::Bytes, Error = hyper::Error>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            // Do extra async work here...
            let uri = req.uri().clone();
            let response = inner.call(req).await?;

            if let Some(grpc_status) = response.headers().get("grpc-status") {
                if let Ok(header_v) = grpc_status.to_str() {
                    if let Ok(idx) = header_v.parse::<i32>() {
                        let code = tonic::Code::from(idx);

                        match code {
                            tonic::Code::Ok => (),
                            tonic::Code::NotFound => (), // happens all the time, not interesting
                            code => {
                                let urlencoded_message = response
                                    .headers()
                                    .get("grpc-message")
                                    .map(|e| e.to_str().unwrap_or_default())
                                    .unwrap_or_default();

                                let decoded = urlencoding::decode(urlencoded_message).unwrap_or(
                                    std::borrow::Cow::Owned(urlencoded_message.to_string()),
                                );
                                tracing::warn!(
                                    "Requested: {} grpc response: {}, message: {}",
                                    uri.path(),
                                    code.description(),
                                    decoded
                                )
                            }
                        }
                    }
                }
            }

            Ok(response)
        })
    }
}
