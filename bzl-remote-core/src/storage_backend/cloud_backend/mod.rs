use bazelfe_protos::build::bazel::remote::execution::v2::{self as execution};

mod s3;
mod s3_cas;

use futures;
use redis::AsyncCommands;
use tokio::io::AsyncReadExt;

use std::collections::HashSet;

use std::path::PathBuf;

use std::sync::Arc;

use crate::config::cache_service_config::CloudBackendConfig;
use crate::hash::sha256_value::Sha256Value;
use crate::storage_backend::local_disk_backend::DataLocation;

use s3::S3;
use s3_cas::S3Cas;

use super::api::insert_action_result_to_cas;
use super::api::DataReturnTpe;
use super::LocalDiskStorageBackend;
use super::StorageBackend;
use super::StorageBackendError;
use super::UploadType;

impl From<redis::RedisError> for StorageBackendError {
    fn from(e: redis::RedisError) -> Self {
        StorageBackendError::Unknown(format!("{:?}", e))
    }
}

// hset/hget for presence are because in the hot path sending all of our digests to redis is expensive.
// if we presume with half the bits we won't have regular collisions we will drop our network traffic in half
// once we expire these keys before they get too many collisions we can control the size
//
// if we get many collisions then this is would be worse!

pub struct CloudBackend {
    _config: CloudBackendConfig,
    local_disk_backend: LocalDiskStorageBackend,
    s3_cas: S3Cas,
    s3_kv_store: S3Cas,
    io_path: PathBuf,
    ac_redis: redis::aio::ConnectionManager,
    kv_redis: redis::aio::ConnectionManager,
    cas_redis: redis::aio::ConnectionManager,
    presence_redis: redis::aio::ConnectionManager,
}
impl std::fmt::Debug for CloudBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CloudBackend")
            .field("_config", &self._config)
            .field("local_disk_backend", &self.local_disk_backend)
            .field("s3_cas", &self.s3_cas)
            .field("s3_kv_store", &self.s3_kv_store)
            .field("io_path", &self.io_path)
            .finish()
    }
}

impl CloudBackend {
    pub async fn new(config: &CloudBackendConfig) -> Result<Self, Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&config.local_working_path)?;

        let s3 = Arc::new(S3::new(&config.s3_region, &config.s3_bucket, &config.s3_prefix).await?);

        let s3_cas = S3Cas::new(&s3, "cas").await?;
        let s3_kv_store = S3Cas::new(&s3, "kvd").await?;

        let loc_root = PathBuf::from(&config.local_working_path);
        let local_disk_cache_path = loc_root.join("disk_cache");
        let io_path = loc_root.join("io_tmp");
        std::fs::create_dir_all(&io_path)?;
        let local_disk = LocalDiskStorageBackend::open(&local_disk_cache_path)?;

        let ac_redis = redis::aio::ConnectionManager::new(redis::Client::open(format!(
            "redis://{}/10",
            &config.redis_host
        ))?)
        .await?;
        let cas_redis = redis::aio::ConnectionManager::new(redis::Client::open(format!(
            "redis://{}/11",
            &config.redis_host
        ))?)
        .await?;

        let presence_redis = redis::aio::ConnectionManager::new(redis::Client::open(format!(
            "redis://{}/12",
            &config.redis_host
        ))?)
        .await?;

        let kv_redis = redis::aio::ConnectionManager::new(redis::Client::open(format!(
            "redis://{}/13",
            &config.redis_host
        ))?)
        .await?;

        Ok(Self {
            _config: config.clone(),
            s3_cas,
            s3_kv_store,
            local_disk_backend: local_disk,
            io_path,
            ac_redis,
            cas_redis,
            presence_redis,
            kv_redis,
        })
    }

    // true if we do not store on redis
    fn is_s3_only(&self, digest: &execution::Digest) -> bool {
        digest.size_bytes > 1024 * 256
    }

    async fn kv_get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        let mut connection = self.kv_redis.clone();
        let redis_value: Option<Vec<u8>> = connection.get(key).await?;

        let digest: execution::Digest = key.try_into()?;
        if let Some(returned_value) = redis_value {
            if !self.s3_kv_store.exists(&digest).await? {
                self.s3_kv_store
                    .upload_bytes(returned_value.clone(), &digest)
                    .await?;
            }
            Ok(Some(returned_value))
        } else {
            if !self.s3_kv_store.exists(&digest).await? {
                // it does not exist on s3
                Ok(None)
            } else {
                let local_data = self.s3_kv_store.download_bytes(&digest).await?;
                Ok(Some(local_data))
            }
        }
    }

    async fn kv_put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        let mut connection = self.kv_redis.clone();

        let digest: execution::Digest = key.try_into()?;
        if !self.s3_kv_store.exists(&digest).await? {
            self.s3_kv_store
                .upload_bytes(value.to_vec(), &digest)
                .await?;
        }

        connection.set(key, value).await?;

        Ok(())
    }

    async fn ac_get(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<execution::ActionResult>, StorageBackendError> {
        use prost::Message;

        let hash_bytes = digest.hash.as_bytes();
        let mut connection = self.ac_redis.clone();
        let redis_value: Option<Vec<u8>> = connection.get(hash_bytes).await?;
        if let Some(v) = redis_value {
            Ok(Some(execution::ActionResult::decode(&v[..])?))
        } else {
            Ok(None)
        }
    }

    async fn fetch_size_from_redis(
        &self,
        hash: &String,
    ) -> Result<Option<u64>, StorageBackendError> {
        let mut connection = self.presence_redis.clone();
        let hash_bytes = hash.as_bytes();
        let mini_key = &hash_bytes[0..12];

        let r: Option<u64> = connection.hget(mini_key, hash_bytes).await?;
        Ok(r)
    }

    async fn ac_put(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<(), StorageBackendError> {
        use prost::Message;

        let hash_bytes = digest.hash.as_bytes();
        let mut connection = self.ac_redis.clone();

        connection
            .set(hash_bytes, action_result.encode_to_vec())
            .await?;

        Ok(())
    }

    async fn redis_cas_note_exists(
        &self,
        digest: &execution::Digest,
        best_effort: bool,
    ) -> Result<(), StorageBackendError> {
        let digest = digest.clone();

        let mut connection = self.presence_redis.clone();

        let update_closure = async move {
            let mini_key = &digest.hash.as_bytes()[0..12];
            let hash_bytes = digest.hash.as_bytes();

            redis::pipe()
                .atomic()
                .hset(mini_key, hash_bytes, digest.size_bytes as u64)
                .expire(mini_key, 60 * 60 * 24 * 30)
                .query_async(&mut connection)
                .await
        };

        if !best_effort {
            update_closure.await?;
        } else {
            let _ = tokio::spawn(update_closure);
        };

        Ok(())
    }

    async fn filter_not_s3_exists_from_redis(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        let mut connection = self.presence_redis.clone();

        let mut pipeline = redis::pipe();

        for d in digests.iter() {
            pipeline.hgetall(&d.hash.as_bytes()[0..12]);
        }

        let results: Vec<Option<std::collections::HashMap<String, u32>>> =
            pipeline.query_async(&mut connection).await?;

        let mut merged: HashSet<String> = Default::default();
        for v in results.into_iter().flatten() {
            for (k, _v) in v {
                merged.insert(k);
            }
        }

        let mut res: Vec<execution::Digest> = Vec::with_capacity(digests.len());
        for ele in digests.drain(..) {
            if !merged.contains(&ele.hash) {
                res.push(ele);
            }
        }
        std::mem::swap(digests, &mut res);

        Ok(())
    }

    async fn assert_inbound_digest_match(
        &self,
        digest: &execution::Digest,
        data: &UploadType,
    ) -> Result<(), StorageBackendError> {
        // this is cheap, just a string copy
        let expected_digest = Sha256Value::try_from(digest)?;
        let actual_value = match data {
            UploadType::InMemory(d) => Sha256Value::try_from(&d[..])?,
            UploadType::OnDisk(f) => Sha256Value::from_path(f).await?,
        };
        if actual_value != expected_digest {
            return Err(StorageBackendError::InvalidDigestForDataInbound(
                expected_digest,
                actual_value,
            ));
        }
        Ok(())
    }

    async fn assert_outbound_digest_match_file<'a>(
        &'a self,
        digest: &'a execution::Digest,
        location: &'a std::path::Path,
        source: &'static str,
    ) -> Result<(), StorageBackendError> {
        // this is cheap, just copying a string
        let expected_digest = Sha256Value::try_from(digest)?;

        // size check is cheaper so do it first
        let size_fut = async {
            match tokio::fs::File::open(&location).await {
                Ok(f) => f.metadata().await.map(|m| m.len()),
                Err(io_err) => Err(io_err),
            }
        };

        let size = size_fut.await.map_err(|e| {
            StorageBackendError::ErrorAndMessage(
                "Error attempting to open file".to_string(),
                Box::new(e),
            )
        })?;
        if (size as i64) != digest.size_bytes {
            return Err(StorageBackendError::InvalidSizeForDataOutbound(
                expected_digest,
                digest.size_bytes,
                size,
            ));
        }
        let actual_sha = Sha256Value::from_path(location).await.map_err(|e| {
            StorageBackendError::ErrorAndMessage(
                "Error attempting to open file".to_string(),
                Box::new(e),
            )
        })?;

        if actual_sha != expected_digest {
            tracing::error!(
                "[{}] Outbound digest match failure, looking at {:#?} - built {:?}",
                source,
                digest,
                actual_sha
            );
            return Err(StorageBackendError::InvalidDigestForDataOutbound(
                expected_digest,
                actual_sha,
            ));
        }
        Ok(())
    }

    fn assert_outbound_digest_match(
        &self,
        digest: &execution::Digest,
        data: &[u8],
        source: &'static str,
    ) -> Result<(), StorageBackendError> {
        // this is cheap, just copying a string
        let expected_digest = Sha256Value::try_from(digest)?;
        // size check is cheaper so do it first
        if (data.len() as i64) != digest.size_bytes {
            return Err(StorageBackendError::InvalidSizeForDataOutbound(
                expected_digest,
                digest.size_bytes,
                data.len() as u64,
            ));
        }
        let actual_value = Sha256Value::try_from(data)?;
        if actual_value != expected_digest {
            tracing::error!(
                "[{}] Outbound digest match failure, looking at {:?}(len: {}) - built {:?}(len: {})",
                source,
                digest,
                digest.size_bytes,
                actual_value,
                data.len()
            );
            return Err(StorageBackendError::InvalidDigestForDataOutbound(
                expected_digest,
                actual_value,
            ));
        }
        Ok(())
    }

    async fn redis_cas_put(
        &self,
        digest: &execution::Digest,
        data: &[u8],
    ) -> Result<(), StorageBackendError> {
        let hash_bytes = digest.hash.as_bytes();
        let mut connection = self.cas_redis.clone();

        redis::pipe()
            .atomic()
            .set(hash_bytes, data)
            .expire(hash_bytes, 60 * 60 * 24 * 3)
            .query_async(&mut connection)
            .await?;

        Ok(())
    }

    // this is a cache of what exists *on s3* which is used
    // for filtering. We use a smaller key (4 bytes) which means
    // we should access more frequently and keep this data in cache longer
    async fn redis_s3_cas_exists(
        &self,
        digest: &execution::Digest,
    ) -> Result<bool, StorageBackendError> {
        let mut digests = vec![digest.clone()];
        self.filter_not_s3_exists_from_redis(&mut digests).await?;

        Ok(digests.is_empty())
    }

    /**
     * This the redis exists command on the full digest key to test if the
     * full value exists in redis
     */
    async fn redis_cas_exists(
        &self,
        digest: &execution::Digest,
    ) -> Result<bool, StorageBackendError> {
        if self.is_s3_only(digest) {
            Ok(false)
        } else {
            let hash_bytes = digest.hash.as_bytes();
            // this clone is so we can use a mut ref to connection.
            // probably self should but a mut ref.
            let mut connection = self.cas_redis.clone();
            Ok(connection.exists(hash_bytes).await?)
        }
    }

    async fn redis_cas_get(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Vec<u8>>, StorageBackendError> {
        if self.is_s3_only(digest) {
            Ok(None)
        } else {
            let hash_bytes = digest.hash.as_bytes();
            let mut connection = self.cas_redis.clone();
            let redis_value: Option<Vec<u8>> = connection.get(hash_bytes).await?;

            Ok(redis_value)
        }
    }

    // fetch a digest, writing it a given target_path. If this fails, we will need to clean it up
    async fn fetch_insert(
        &self,
        target_path: &PathBuf,
        digest: &execution::Digest,
    ) -> Result<(), StorageBackendError> {
        self.s3_cas
            .download(target_path, digest)
            .await
            .map_err(|e| {
                tracing::warn!(
                    "Unable to download digest: {:#?} from s3, but it was reported present",
                    digest
                );
                e
            })?;

        // sanity check that hash matches
        self.assert_outbound_digest_match_file(digest, target_path, "Download from s3")
            .await?;

        self.local_disk_backend
            .insert(digest, UploadType::OnDisk(target_path.clone()))
            .await?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl StorageBackend for CloudBackend {
    async fn get_kv(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageBackendError> {
        if let Some(r) = self.local_disk_backend.get_kv(key).await? {
            return Ok(Some(r));
        }

        Ok(self.kv_get(key).await?)
    }

    async fn put_kv(&self, key: &[u8], value: &[u8]) -> Result<(), StorageBackendError> {
        self.local_disk_backend.put_kv(key, value).await?;

        self.kv_put(key, value).await
    }

    async fn get_action_result(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<Arc<execution::ActionResult>>, StorageBackendError> {
        if let Some(r) = self.local_disk_backend.get_action_result(digest).await? {
            return Ok(Some(r));
        }

        if let Some(r) = self.ac_get(digest).await? {
            Ok(Some(Arc::new(r)))
        } else {
            Ok(None)
        }
    }
    async fn put_action_result(
        &self,
        digest: &execution::Digest,
        action_result: &execution::ActionResult,
    ) -> Result<execution::Digest, StorageBackendError> {
        let action_result_digest = insert_action_result_to_cas(self, action_result).await?;

        self.local_disk_backend
            .put_action_result(digest, action_result)
            .await?;

        self.ac_put(digest, action_result).await?;

        Ok(action_result_digest)
    }

    async fn build_digest_from_hash_if_present(
        &self,
        hash: &String,
    ) -> Result<Option<execution::Digest>, StorageBackendError> {
        if let Some(r) = self
            .local_disk_backend
            .build_digest_from_hash_if_present(hash)
            .await?
        {
            return Ok(Some(r));
        }

        if let Some(siz) = self.fetch_size_from_redis(hash).await? {
            Ok(Some(execution::Digest {
                size_bytes: siz as i64,
                hash: hash.clone(),
            }))
        } else {
            Ok(None)
        }
    }

    // this is weak: if you are in the list at the end, you *may* still exist on s3 (we avoid s3 check here)
    // but if you are are gone, you definitely exist on s3
    async fn cas_filter_for_missing(
        &self,
        digests: &mut Vec<execution::Digest>,
    ) -> Result<(), StorageBackendError> {
        self.local_disk_backend
            .cas_filter_for_missing(digests)
            .await?;

        self.filter_not_s3_exists_from_redis(digests).await?;

        Ok(())
    }

    // strong guarantee: serial check local => redis => s3
    async fn cas_exists(&self, digest: &execution::Digest) -> Result<bool, StorageBackendError> {
        if self.local_disk_backend.cas_exists(digest).await? {
            return Ok(true);
        }

        if self.redis_s3_cas_exists(digest).await? {
            return Ok(true);
        }

        // it may still be on s3 and fallen out of redis cache
        Ok(self.s3_cas.exists(digest).await?)
    }

    // goal: once you get a success, we have durable stored into the s3 cas.
    //       exists returns true if and only if it is on s3 (check in the reverse order we write)
    //       we want for local or redis presence implies s3 existance, absence on local or redis and it *may* be on s3
    //       we can't guarantee that with local or redis, since redis could have evicted and local may have restarted
    // we fetch/read sequentially to avoid load on s3. local then redis then s3
    // after writing to s3, we could operate locally and with redis in parallel, but currently do not.
    async fn cas_insert(
        &self,
        digest: &execution::Digest,
        data: UploadType,
    ) -> Result<(), StorageBackendError> {
        if self.local_disk_backend.cas_exists(digest).await? {
            // we already have the data
            return Ok(());
        }

        // We are expecting to write this ~somewhere.
        self.assert_inbound_digest_match(digest, &data).await?;

        // if this digest points to something small enough to store in redis
        // AND it is actually present in redis, we can just insert into redis
        // and continue
        if self.redis_cas_exists(digest).await? {
            self.local_disk_backend.insert(digest, data).await?;
            return Ok(());
        }

        if !(self.redis_s3_cas_exists(digest).await? || self.s3_cas.exists(digest).await?) {
            // since s3 is slower and more expensive to talk to than redis, we only check it
            // as a last resort
            //
            // we have to insert into s3 *before* writing into our non-durable caching layers
            // since we don't want a file to appear to exist in cache but not be durably stored
            match &data {
                UploadType::OnDisk(path) => self.s3_cas.upload(path, digest).await?,
                UploadType::InMemory(byte_vec) =>
                // s3 takes ownership of the data, so we have to clone it here unfortunately
                {
                    self.s3_cas.upload_bytes(byte_vec.clone(), digest).await?
                }
            }
        }
        // we have to insert locally and to redis, but the order doesn't matter
        // so it is more convenient to insert locally first
        let inserted_data = self.local_disk_backend.insert(digest, data).await?;
        if !self.is_s3_only(digest) {
            // the data is in s3 and locally, and should be on redis, but isn't
            let upload_future = async {
                match inserted_data {
                    DataLocation::OnDisk(p) => {
                        // this should be in redis, but isn't
                        let mut f = tokio::fs::File::open(&p).await.map_err(|e| {
                            StorageBackendError::ErrorAndMessage(
                                "Error attempting to open file".to_string(),
                                Box::new(e),
                            )
                        })?;
                        let mut data = Vec::default();
                        f.read_to_end(&mut data).await.map_err(|e| {
                            StorageBackendError::ErrorAndMessage(
                                "Error attempting to read file into memory".to_string(),
                                Box::new(e),
                            )
                        })?;
                        Ok(self.redis_cas_put(digest, &data).await?)
                    }
                    DataLocation::InMemory => {
                        let data = self.local_disk_backend.cas_to_vec(digest).await?;
                        if let Some(d) = data {
                            Ok(self.redis_cas_put(digest, &d).await?)
                        } else {
                            Err(StorageBackendError::Unknown(String::from(
                                "Just uploaded data wasn't present?",
                            )))
                        }
                    }
                }
            };

            // we can upload to redis and record that the file exists at the same time
            futures::try_join!(upload_future, self.redis_cas_note_exists(digest, false)).map(|_| ())
        } else {
            self.redis_cas_note_exists(digest, false).await
        }
    }

    // idea is check in same order as cas_exists (local, redis, s3)
    // don't need the redis existance, just check with s3 on redis get failure
    async fn cas_get_data(
        &self,
        digest: &execution::Digest,
    ) -> Result<Option<DataReturnTpe>, StorageBackendError> {
        if let Some(ret) = self.local_disk_backend.cas_get_data(digest).await? {
            let _ = self.assert_outbound_digest_match(
                digest,
                ret.as_ref().as_ref(),
                "previously in local disk cache",
            )?;

            return Ok(Some(ret));
        }
        // we didn't find it locally, but redis or s3 may have it.
        let mut redis_put = false;

        if let Some(v) = self.redis_cas_get(digest).await? {
            // the data is present in redis (which implies also s3), but not locally
            let _ = self.assert_outbound_digest_match(digest, &v, "redis")?;

            self.local_disk_backend
                .insert(digest, UploadType::InMemory(v))
                .await?;
        } else if !self.s3_cas.exists(digest).await? {
            // it does not exist on s3
            return Ok(None);
        } else {
            // the data isn't present locally or in redis, but it *is* on s3
            // we fetch it onto a new temp path
            redis_put = !self.is_s3_only(digest);
            let target_path = self
                .io_path
                .join(format!("{}.tmp", rand::random::<usize>()));

            if let Err(cause) = self.fetch_insert(&target_path, digest).await {
                if target_path.exists() {
                    let _ = std::fs::remove_file(&target_path).map_err(|e| {
                        tracing::warn!("Unable to remove {:#?} on failure {:#?}, due to remove_file error {:#?}", target_path, cause, e)
                    });
                }
                return Err(cause);
            }
        }

        // now that we have reinserted locally, it must be there unless things are really broken
        if let Some(ret) = self.local_disk_backend.cas_get_data(digest).await? {
            let data = ret.as_ref().as_ref();
            let _ = self.assert_outbound_digest_match(digest, data, "local cache after fetch")?;
            if redis_put {
                // the data should be in redis but isn't
                self.redis_cas_put(digest, data).await?;
            }

            self.redis_cas_note_exists(digest, true).await?;

            return Ok(Some(ret));
        }

        Err(StorageBackendError::Unknown(String::from(
            "Bad state reached, just fetched and installed file but not present",
        )))
    }
}
