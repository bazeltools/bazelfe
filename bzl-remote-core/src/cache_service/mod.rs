pub trait OptionAsStatusError<T> {
    fn take_or_error(&mut self) -> Result<T, tonic::Status>;
}

impl<T> OptionAsStatusError<T> for Option<T>
where
    T: std::fmt::Debug,
{
    fn take_or_error(&mut self) -> Result<T, tonic::Status> {
        if let Some(r) = self.take() {
            Ok(r)
        } else {
            Err(tonic::Status::failed_precondition(format!(
                "An expected set value was empty, looking at {:#?}",
                self
            )))
        }
    }
}

pub mod action_cache_service;
pub mod bytestream_service;
pub mod capabilities_service;
pub mod content_addressable_storage_service;
pub mod fetch_service;
pub mod http_endpoint;
pub mod metadata_service;

use std::sync::Arc;

use tracing::info;

use crate::config::Config;
pub async fn storage_backend_from_config(
    config: &Config,
) -> Result<Arc<dyn crate::storage_backend::StorageBackend>, Box<dyn std::error::Error>> {
    match &config.cache_config.cache_backend {
        crate::config::cache_service_config::CacheServiceStorage::OnLocalDisk { path } => {
            info!("Setup OnLocalDisk backend for launch in path {:#?}", path);
            Ok(Arc::new(
                crate::storage_backend::LocalDiskStorageBackend::open(path)?,
            ))
        }
        crate::config::cache_service_config::CacheServiceStorage::InMemory {} => {
            info!("Setup InMemory backend for launch");
            Ok(Arc::new(
                crate::storage_backend::InMemoryStorageBackend::default(),
            ))
        }
        crate::config::cache_service_config::CacheServiceStorage::CloudBackend(cfg) => {
            info!("Setup CloudBackend for launch: {:#?}", cfg);
            Ok(Arc::new(
                crate::storage_backend::CloudBackend::new(cfg).await?,
            ))
        }
    }
}

impl From<crate::storage_backend::StorageBackendError> for tonic::Status {
    fn from(backend_err: crate::storage_backend::StorageBackendError) -> Self {
        match backend_err {
            crate::storage_backend::StorageBackendError::Unknown(message) => {
                tracing::error!("Unknown that isn't user actionable {}", message);
                tonic::Status::internal(message)
            }
            crate::storage_backend::StorageBackendError::InternalError(message) => {
                tracing::error!("Internal error that isn't actionable {}", message);
                tonic::Status::internal(
                    "Application encountered an internal error, please try again",
                )
            }
            crate::storage_backend::StorageBackendError::IOError(message) => {
                tracing::error!(
                    "Triggered an I/O error not handled more gracefully {}",
                    message
                );
                tonic::Status::internal(
                    "Application encountered an internal error, please try again",
                )
            }
            crate::storage_backend::StorageBackendError::ErrorAndMessage(message, error) => {
                tracing::error!(
                    "A generic error with message {}, error: {:?}",
                    message,
                    error
                );
                tonic::Status::internal(
                    "Application encountered an internal error, please try again",
                )
            }
            crate::storage_backend::StorageBackendError::InvalidSha256Value(invalid_vale) => {
                tracing::warn!(
                    "Attempted to parse an invalid sha256 value error: {:?}",
                    invalid_vale
                );
                tonic::Status::invalid_argument(format!(
                    "Application passed an invalid sha256 value, probably in a digest: {:#?}",
                    invalid_vale
                ))
            }

            crate::storage_backend::StorageBackendError::InvalidDigestForDataInbound(
                expected,
                actual,
            ) => {
                tracing::warn!(
                    "Application uploaded data which failed sha256 check, expected: {:?}, actual: {:?}",
                    expected,
                    actual
                );
                tonic::Status::invalid_argument(
                    format!("Application uploaded data which failed sha256 check, expected: {:?}, actual: {:?}",
                    expected,
                    actual)
                )
            }
            crate::storage_backend::StorageBackendError::InvalidDigestForDataOutbound(
                expected,
                actual,
            ) => {
                tracing::error!(
                    "Outbound data missmatch on sha values expected: {:#?} , actual: {:#?}",
                    expected,
                    actual
                );
                tonic::Status::internal(format!(
                    "Outbound data missmatch on sha values expected: {:#?} , actual: {:#?}",
                    expected, actual
                ))
            }
            crate::storage_backend::StorageBackendError::InvalidSizeForDataInbound(
                expected_sha,
                expected,
                actual,
            ) => {
                tracing::warn!(
                    "Application uploaded data with expected sha {:?} which had incorrect size, expected: {:?}, actual: {:?}",
                    expected_sha,
                    expected,
                    actual
                );
                tonic::Status::invalid_argument(
                    format!("Application uploaded data with expected sha {:?} which had incorrect size, expected: {:?}, actual: {:?}",
                    expected_sha,
                    expected,
                    actual)
                )
            }
            crate::storage_backend::StorageBackendError::InvalidSizeForDataOutbound(
                expected_sha,
                expected,
                actual,
            ) => {
                tracing::warn!(
                    "Application uploaded data with expected sha {:?} which had incorrect size, expected: {:?}, actual: {:?}",
                    expected_sha,
                    expected,
                    actual
                );
                tonic::Status::invalid_argument(
                    format!("Application uploaded data with expected sha {:?} which had incorrect size, expected: {:?}, actual: {:?}",
                    expected_sha,
                    expected,
                    actual)
                )
            }
        }
    }
}
