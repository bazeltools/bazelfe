use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
use execution::action_cache_server;

use tonic::{Request, Response};

use super::OptionAsStatusError;
use crate::storage_backend::StorageBackend;

#[derive(Debug)]
pub struct ActionCacheService<T>
where
    T: StorageBackend + 'static,
{
    storage_backend: T,
}

impl<T> ActionCacheService<T>
where
    T: StorageBackend + 'static + Sized,
{
    pub fn new(storage_backend: T) -> ActionCacheService<T> {
        ActionCacheService { storage_backend }
    }
}

#[tonic::async_trait]
impl<T> action_cache_server::ActionCache for ActionCacheService<T>
where
    T: StorageBackend + 'static,
{
    async fn get_action_result(
        &self,
        request: Request<execution::GetActionResultRequest>,
    ) -> Result<tonic::Response<execution::ActionResult>, tonic::Status> {
        tracing::debug!("Received get_action_result for {:#?}", request.get_ref());

        let mut request = request.into_inner();
        let action_digest = request.action_digest.take_or_error()?;

        if let Some(action_r) = self
            .storage_backend
            .get_action_result(&action_digest)
            .await?
        {
            Ok(Response::new(action_r.as_ref().to_owned()))
        } else {
            tracing::info!("Cache miss for digest : {}", action_digest.hash);
            Err(tonic::Status::not_found("Unable to find in cache"))
        }
    }

    async fn update_action_result(
        &self,
        request: Request<execution::UpdateActionResultRequest>,
    ) -> Result<tonic::Response<execution::ActionResult>, tonic::Status> {
        let mut request = request.into_inner();
        let action_digest = request.action_digest.take_or_error()?;
        let action_result = request.action_result.take_or_error()?;
        let action_result_digest = self
            .storage_backend
            .put_action_result(&action_digest, &action_result)
            .await?;

        // The output path is one of the best local identifiers as to the action this comes from
        // the command would be good too, but we would have to find that via the action.
        let mut out_f: Vec<String> = action_result
            .output_files
            .iter()
            .map(|e| e.path.clone())
            .collect();

        out_f.extend(
            action_result
                .output_directories
                .iter()
                .map(|e| e.path.clone()),
        );
        out_f.sort();

        let action_name = out_f
            .into_iter()
            .next()
            .unwrap_or_else(|| "No output file".to_string());

        tracing::info!(
            "Inserted Action {} for {} , action result: {}",
            action_digest.hash,
            action_name,
            action_result_digest.hash
        );

        Ok(Response::new(action_result))
    }
}
