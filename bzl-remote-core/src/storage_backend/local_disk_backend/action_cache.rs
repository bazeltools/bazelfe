use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
use prost::{DecodeError, Message};

use super::kv_store::KvStore;

#[derive(PartialEq, Hash, Eq, Debug)]
struct ByteWrapper(Vec<u8>);

impl From<sled::Error> for super::StorageBackendError {
    fn from(e: sled::Error) -> Self {
        super::StorageBackendError::InternalError(Box::new(e))
    }
}

impl From<DecodeError> for super::StorageBackendError {
    fn from(e: DecodeError) -> Self {
        super::StorageBackendError::InternalError(Box::new(e))
    }
}

impl std::borrow::Borrow<[u8]> for ByteWrapper {
    fn borrow(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(Debug)]
pub struct ActionCache(KvStore);

impl ActionCache {
    pub fn new(sled_connection: &sled::Db) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(ActionCache(super::kv_store::KvStore::new(
            sled_connection,
            "ac",
        )?))
    }

    pub fn get_action(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<execution::ActionResult>, super::StorageBackendError> {
        let key = digest.hash.as_bytes();
        match self.0.get(key)? {
            Some(tree_tpe) => Ok(Some(execution::ActionResult::decode(tree_tpe.as_ref())?)),
            None => Ok(None),
        }
    }

    pub fn insert(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<(), super::StorageBackendError> {
        let key = digest.hash.as_bytes();
        let value = action_result.encode_to_vec();
        self.0.insert(key, &value)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::*;

    struct DevSled(TempDir, sled::Db);

    fn setup_temp_sled() -> Result<DevSled, Box<dyn std::error::Error>> {
        let tmp_dir = tempdir().unwrap();

        let sled_connection = sled::open(tmp_dir.path().join("sled"))?;

        Ok(DevSled(tmp_dir, sled_connection))
    }

    #[tokio::test]
    async fn test_action_cache() -> Result<(), Box<dyn std::error::Error>> {
        let dev_sled = setup_temp_sled()?;

        let action_cache = ActionCache::new(&dev_sled.1)?;

        let digest = execution::Digest {
            hash: String::from("Hello world"),
            ..Default::default()
        };
        assert_eq!(action_cache.get_action(&digest)?, None);

        let action_result = execution::ActionResult::default();
        action_cache.insert(&digest, &action_result)?;
        assert_eq!(action_cache.get_action(&digest)?, Some(action_result));
        Ok(())
    }
}
