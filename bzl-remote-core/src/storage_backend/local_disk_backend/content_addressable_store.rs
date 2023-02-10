use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

use std::hash::Hash;

use std::path::PathBuf;
use std::sync::Arc;

use super::super::StorageBackendError;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum InMemoryLutTreeType {
    OnDisk,
    InDB(u64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataLocation {
    OnDisk(PathBuf),
    InMemory,
}

#[derive(PartialEq, Hash, Eq, Debug)]
struct ByteWrapper(Vec<u8>);

#[derive(Debug)]
struct MemoryMappedFile {
    _f: std::fs::File,
    mmap: memmap2::Mmap,
}

impl AsRef<[u8]> for MemoryMappedFile {
    fn as_ref(&self) -> &[u8] {
        &self.mmap
    }
}

pub struct ArcDynBox(Arc<dyn AsRef<[u8]> + Send + Sync>);
impl AsRef<[u8]> for ArcDynBox {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref().as_ref()
    }
}

// from the sled docs for incrementing
fn sled_u64_increment(old: Option<&[u8]>) -> Option<Vec<u8>> {
    let number = match old {
        Some(bytes) => {
            let array: [u8; 8] = bytes.try_into().unwrap();
            let number = u64::from_be_bytes(array);
            number + 1
        }
        None => 1,
    };

    Some(number.to_be_bytes().to_vec())
}

#[derive(Debug)]
pub struct ContentAddressableStore {
    lut_tree: sled::Tree,
    small_file_tree: sled::Tree,
    large_blob_path: PathBuf,
}

impl ContentAddressableStore {
    pub fn new(
        sled_connection: &sled::Db,
        large_blob_path: PathBuf,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let lut_tree = sled_connection.open_tree("cas_lut")?;
        let small_file_tree = sled_connection.open_tree("cas_small")?;
        Ok(Self {
            lut_tree,
            small_file_tree,
            large_blob_path,
        })
    }

    fn store_metadata(
        &self,
        digest: &execution::Digest,
        metadata: InMemoryLutTreeType,
    ) -> Result<(), super::StorageBackendError> {
        let key = digest.hash.as_bytes();

        let v = match metadata {
            InMemoryLutTreeType::OnDisk => 0,
            InMemoryLutTreeType::InDB(v) => v,
        };

        self.lut_tree.insert(key, &v.to_be_bytes())?;
        Ok(())
    }

    pub async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        if let Some(metadata) = self.get_metadata(&hash)? {
            match metadata {
                InMemoryLutTreeType::OnDisk => {
                    let metadata = self.expected_path_hash(hash).metadata().map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "Error attempting to get hash entry metadata".to_string(),
                            Box::new(e),
                        )
                    })?;
                    Ok(Some(execution::Digest {
                        size_bytes: metadata.len() as i64,
                        hash: hash.clone(),
                    }))
                }
                InMemoryLutTreeType::InDB(idx) => Ok(Some(execution::Digest {
                    size_bytes: self.get_from_small_db(idx)?.len() as i64,
                    hash: hash.clone(),
                })),
            }
        } else {
            Ok(None)
        }
    }

    fn expected_path_hash(&self, hash: &String) -> PathBuf {
        self.large_blob_path.join(&hash)
    }

    fn expected_path(&self, digest: &execution::Digest) -> PathBuf {
        self.expected_path_hash(&digest.hash)
    }

    fn get_location(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataLocation>, super::StorageBackendError> {
        if let Some(metadata) = self.get_metadata(&digest.hash)? {
            Ok(Some(match metadata {
                InMemoryLutTreeType::OnDisk => DataLocation::OnDisk(self.expected_path(digest)),
                InMemoryLutTreeType::InDB(_) => DataLocation::InMemory,
            }))
        } else {
            Ok(None)
        }
    }
    fn get_metadata(
        &self,
        hash: &String,
    ) -> Result<Option<InMemoryLutTreeType>, super::StorageBackendError> {
        let key = hash.as_bytes();
        match self.lut_tree.get(key)? {
            Some(tree_tpe) => {
                let mut bytes = [0; 8];
                bytes[..].copy_from_slice(&tree_tpe);
                let v = u64::from_be_bytes(bytes);

                let o = if v == 0 {
                    InMemoryLutTreeType::OnDisk
                } else {
                    InMemoryLutTreeType::InDB(v)
                };
                Ok(Some(o))
            }
            None => Ok(None),
        }
    }

    pub fn exists(&self, digest: &execution::Digest) -> Result<bool, super::StorageBackendError> {
        Ok(self.get_metadata(&digest.hash)?.is_some())
    }

    fn get_next_small_idx(&self) -> Result<u64, StorageBackendError> {
        let k = u64::MAX.to_be_bytes();

        let next_v = self
            .small_file_tree
            .update_and_fetch(k, sled_u64_increment)?;

        match next_v {
            None => Err(StorageBackendError::Unknown(String::from(
                "We got a none incrementing the slab counter",
            ))),
            Some(v) => {
                let mut bytes = [0; 8];
                bytes[..].copy_from_slice(&v);
                Ok(u64::from_be_bytes(bytes))
            }
        }
    }

    fn get_from_small_db(&self, idx: u64) -> Result<Vec<u8>, StorageBackendError> {
        let key = idx.to_be_bytes();
        match self.small_file_tree.get(key)? {
            Some(tree_tpe) => {
                Ok(tree_tpe.to_vec())
            }
            None => Err(StorageBackendError::Unknown(String::from("Shouldn't be possible, but we found an idx in the outer map for a small value, but it not present in the tree")))
        }
    }

    fn get_from_disk(
        &self,
        digest: &execution::Digest,
    ) -> Result<MemoryMappedFile, StorageBackendError> {
        let p = self.large_blob_path.join(&digest.hash);
        if !(p.exists()) {
            return Err(StorageBackendError::Unknown(String::from("Shouldn't be possible, the in memory map mentions this file exists, but its not on disk. State is corrupted.")));
        }

        let f = std::fs::File::open(p).map_err(|e| StorageBackendError::InternalError(e.into()))?;

        let mmap = unsafe {
            memmap2::Mmap::map(&f).map_err(|e| StorageBackendError::InternalError(e.into()))?
        };

        Ok(MemoryMappedFile { _f: f, mmap })
    }

    fn is_disk(&self, digest: &execution::Digest) -> bool {
        digest.size_bytes > 1024 * 128
    }

    // note: this takes ownership of any files you pass in
    // they are no longer valid after you call insert
    pub async fn insert(
        &self,
        digest: &execution::Digest,
        content: super::super::UploadType,
    ) -> Result<DataLocation, StorageBackendError> {
        if let Some(loc) = self.get_location(digest)? {
            return Ok(loc);
        }

        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;

        let content = match content {
            crate::storage_backend::UploadType::InMemory(m) => {
                if self.is_disk(digest) {
                    let tmp_path = self.large_blob_path.join(format!(
                        "{}_{}.tmp",
                        &digest.hash,
                        rand::random::<usize>()
                    ));
                    let mut f = tokio::fs::File::create(&tmp_path).await.map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "Error attempting to open file".to_string(),
                            Box::new(e),
                        )
                    })?;
                    f.write_all(&m).await.map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "Error attempting to write to file".to_string(),
                            Box::new(e),
                        )
                    })?;
                    f.flush().await.map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "Error attempting to flush file".to_string(),
                            Box::new(e),
                        )
                    })?;
                    drop(f);
                    if std::fs::metadata(&tmp_path).expect("Should work").len()
                        != digest.size_bytes as u64
                    {
                        panic!("Should never be able to happen, we just wrote these files but not enough bytes are written.")
                    }
                    crate::storage_backend::UploadType::OnDisk(tmp_path)
                } else {
                    crate::storage_backend::UploadType::InMemory(m)
                }
            }
            crate::storage_backend::UploadType::OnDisk(path) => {
                if self.is_disk(digest) {
                    crate::storage_backend::UploadType::OnDisk(path)
                } else {
                    let mut f = tokio::fs::File::open(&path).await.map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "Error attempting to open file".to_string(),
                            Box::new(e),
                        )
                    })?;
                    let mut data = Vec::default();
                    f.read_to_end(&mut data).await.map_err(|e| {
                        StorageBackendError::ErrorAndMessage(
                            "ERror attempting to read file into memory".to_string(),
                            Box::new(e),
                        )
                    })?;

                    crate::storage_backend::UploadType::InMemory(data)
                }
            }
        };

        match content {
            crate::storage_backend::UploadType::InMemory(data) => {
                let idx = self.get_next_small_idx()?;

                self.small_file_tree.insert(idx.to_be_bytes(), &data[..])?;
                self.store_metadata(digest, InMemoryLutTreeType::InDB(idx))?;
                Ok(DataLocation::InMemory)
            }
            crate::storage_backend::UploadType::OnDisk(f) => {
                // we either atomically rename or (copy and remove) the original file.
                // it is not the responsibilty of the caller of this function to clean up the file
                let expected_path = self.large_blob_path.join(&digest.hash);
                if expected_path.exists() {
                    return Ok(DataLocation::OnDisk(expected_path));
                }
                // Try rename first, since that is atomic and fast if on the same file system
                if std::fs::rename(&f, &expected_path).is_ok() {
                    self.store_metadata(digest, InMemoryLutTreeType::OnDisk)?;
                    return Ok(DataLocation::OnDisk(expected_path));
                }
                tracing::info!("Rename failed, doing copy");
                let tmp_path = self.large_blob_path.join(format!(
                    "{}_{}.tmp",
                    &digest.hash,
                    rand::random::<usize>()
                ));
                if let Err(e) = std::fs::copy(&f, &tmp_path)
                    .map_err(|e| StorageBackendError::InternalError(Box::new(e)))
                {
                    let _ = std::fs::remove_file(&tmp_path);
                    let _ = std::fs::remove_file(&f);
                    return Err(e);
                }

                // success, remove the original file.
                let _ = std::fs::remove_file(&f);

                std::fs::rename(&tmp_path, &expected_path)
                    .map_err(|e| StorageBackendError::InternalError(Box::new(e)))?;

                self.store_metadata(digest, InMemoryLutTreeType::OnDisk)?;
                Ok(DataLocation::OnDisk(expected_path))
            }
        }
    }

    pub async fn get_to_vec(
        &self,
        digest: &execution::Digest,
    ) -> Result<core::option::Option<Vec<u8>>, StorageBackendError> {
        if let Some(metadata) = self.get_metadata(&digest.hash)? {
            Ok(Some(match metadata {
                InMemoryLutTreeType::OnDisk => self.get_from_disk(digest)?.mmap.to_vec(),
                InMemoryLutTreeType::InDB(idx) => self.get_from_small_db(idx)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn get(
        &self,
        digest: &execution::Digest,
    ) -> Result<core::option::Option<ArcDynBox>, StorageBackendError> {
        if let Some(metadata) = self.get_metadata(&digest.hash)? {
            match metadata {
                InMemoryLutTreeType::OnDisk => {
                    Ok(Some(ArcDynBox(Arc::new(self.get_from_disk(digest)?))))
                }
                InMemoryLutTreeType::InDB(idx) => {
                    Ok(Some(ArcDynBox(Arc::new(self.get_from_small_db(idx)?))))
                }
            }
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{tempdir, TempDir};

    use super::ContentAddressableStore;
    use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

    struct DevSled(TempDir, sled::Db, TempDir);

    fn setup_temp_sled() -> Result<DevSled, Box<dyn std::error::Error>> {
        let tmp_dir = tempdir().unwrap();

        let sled_connection = sled::open(tmp_dir.path().join("sled"))?;

        Ok(DevSled(tmp_dir, sled_connection, tempdir().unwrap()))
    }

    #[tokio::test]
    async fn test_cas() -> Result<(), Box<dyn std::error::Error>> {
        let dev_sled = setup_temp_sled()?;

        let action_cache =
            ContentAddressableStore::new(&dev_sled.1, dev_sled.2.path().to_path_buf())?;

        let digest = execution::Digest {
            hash: String::from("Hello world"),
            ..Default::default()
        };
        assert!(action_cache.get(&digest).await?.is_none());

        // let action_result = execution::ActionResult::default();
        // action_cache.insert(&digest, &action_result)?;
        // assert_eq!(action_cache.get_action(&digest)?, Some(action_result));
        Ok(())
    }
}
