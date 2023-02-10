mod either_body;
pub use either_body::EitherBody;
mod grpc_error_trace_layer;
pub use grpc_error_trace_layer::GrpcErrorTraceLayer;
type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

fn map_option_err<T, U: Into<BoxError>>(err: Option<Result<T, U>>) -> Option<Result<T, BoxError>> {
    err.map(|e| e.map_err(Into::into))
}
