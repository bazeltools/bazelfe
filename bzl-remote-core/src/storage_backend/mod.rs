use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageBackendError {
    // #[error("data store disconnected")]
    // Disconnect(#[from] io::Error),
    // #[error("the data for key `{0}` is not available")]
    // Redaction(String),
    // #[error("invalid header (expected {expected:?}, found {found:?})")]
    // InvalidHeader {
    //     expected: String,
    //     found: String,
    // },
    #[error("Unknown datastore error: {0}")]
    Unknown(String),

    #[error("Unknown IO error: {0}")]
    IOError(#[from] std::io::Error),

    #[error("Unknown datastore error: {0}")]
    InternalError(Box<dyn std::error::Error + Send + Sync>),

    #[error("Unknown Error: {0} {1}")]
    ErrorAndMessage(String, Box<dyn std::error::Error + Send + Sync>),

    #[error("We attempted to parse a sha256 value from a digest but it was invalid: {0}")]
    InvalidSha256Value(#[from] crate::hash::sha256_value::ShaReaderError),

    #[error("In bound uploaded data failed sha256 check, expected {0}, but got {1}")]
    InvalidDigestForDataInbound(
        crate::hash::sha256_value::Sha256Value,
        crate::hash::sha256_value::Sha256Value,
    ),

    #[error("Outbound data failed sha256 check, expected {0}, but got {1}")]
    InvalidDigestForDataOutbound(
        crate::hash::sha256_value::Sha256Value,
        crate::hash::sha256_value::Sha256Value,
    ),

    #[error(
        "In bound uploaded data size mismatch, expected sha {0} with len {1}, but got len {2}"
    )]
    InvalidSizeForDataInbound(crate::hash::sha256_value::Sha256Value, i64, usize),

    #[error("Outbound data size mismatch, expected sha {0} with len {1}, but got len {2}")]
    InvalidSizeForDataOutbound(crate::hash::sha256_value::Sha256Value, i64, u64),
}
// directly build these when we are making internal errors
impl From<bazelfe_protos::digest_utils::DigestExtractError> for StorageBackendError {
    fn from(e: bazelfe_protos::digest_utils::DigestExtractError) -> Self {
        Self::ErrorAndMessage(
            "Unable to extract digest, internal failure".to_string(),
            Box::new(e),
        )
    }
}

mod api;
mod cloud_backend;
mod inmemory_backend;
mod io_helpers;
mod local_disk_backend;

pub use api::StorageBackend;
pub use api::UploadType;
pub use cloud_backend::CloudBackend;
pub use inmemory_backend::InMemoryStorageBackend;
pub use io_helpers::BackendIOHelpers;
pub use local_disk_backend::LocalDiskStorageBackend;
