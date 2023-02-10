#[derive(PartialEq, Hash, Eq, Debug)]
struct ByteWrapper(Vec<u8>);

impl std::borrow::Borrow<[u8]> for ByteWrapper {
    fn borrow(&self) -> &[u8] {
        self.0.as_slice()
    }
}

#[derive(Debug)]
pub struct KvStore {
    tree: sled::Tree,
}
impl KvStore {
    pub fn new(
        sled_connection: &sled::Db,
        tree_name: &'static str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let tree = sled_connection.open_tree(tree_name)?;
        Ok(Self { tree })
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<impl AsRef<[u8]>>, super::StorageBackendError> {
        let r: Option<sled::IVec> = self.tree.get(key)?;
        Ok(r)
    }

    pub fn insert(&self, key: &[u8], value: &[u8]) -> Result<(), super::StorageBackendError> {
        self.tree.insert(key, value)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};
    use prost::Message;
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

        let kv_store = KvStore::new(&dev_sled.1, "test_sled")?;

        let digest = execution::Digest {
            hash: String::from("Hello world"),
            ..Default::default()
        };
        assert!(kv_store.get(&digest.encode_to_vec())?.is_none());

        let action_result = execution::ActionResult::default();
        kv_store.insert(&digest.encode_to_vec(), &action_result.encode_to_vec())?;
        assert_eq!(
            kv_store
                .get(&digest.encode_to_vec())?
                .map(|r| execution::ActionResult::decode(r.as_ref())
                    .expect("Should be able to decode")),
            Some(action_result)
        );
        Ok(())
    }
}
