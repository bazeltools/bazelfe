use crate::build::bazel::remote::execution::v2::{self as execution};
use sha2::Digest;
use sha2::Sha256;
use tokio::io::AsyncReadExt;
use tonic::async_trait;
// use tokio::io::AsyncReadExt;
use std::fmt::Write;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DigestExtractError {
    #[error("Byte slice was the wrong length was {0}, should be 32")]
    WrongBytesLength(usize),
    #[error("Unexpected IO error occured: {0}")]
    UnknownIOError(#[from] std::io::Error),
    #[error("Unexpected Format error occured: {0}")]
    UnknownFmtError(#[from] std::fmt::Error),
}

fn finalized_sha_to_digest(data: &[u8], len: u64) -> Result<execution::Digest, DigestExtractError> {
    if data.len() != 32 {
        return Err(DigestExtractError::WrongBytesLength(data.len()));
    }

    let mut s = String::new();

    for b in data {
        write!(&mut s, "{:02x}", &b)?;
    }

    Ok(execution::Digest {
        hash: s,
        size_bytes: len as i64,
    })
}

#[async_trait]
pub trait AsyncMakeDigestFrom: Sized {
    async fn async_make_digest_from(self) -> Result<execution::Digest, DigestExtractError>;
}

#[async_trait]
impl AsyncMakeDigestFrom for &std::path::Path {
    async fn async_make_digest_from(self) -> Result<execution::Digest, DigestExtractError> {
        let mut f = tokio::fs::File::open(self).await?;

        let mut buffer = vec![0; 1024 * 3];
        let mut hasher = Sha256::new();

        // read up to 10 bytes
        let mut n = 1;
        while n > 0 {
            n = f.read(&mut buffer[..]).await?;

            if n > 0 {
                hasher.update(&buffer[0..n]);
            }
        }

        let len = f.metadata().await.map(|m| m.len())?;

        finalized_sha_to_digest(&hasher.finalize(), len)
    }
}

impl TryFrom<&[u8]> for execution::Digest {
    type Error = DigestExtractError;
    fn try_from(v: &[u8]) -> Result<Self, Self::Error> {
        let mut expected_hasher = Sha256::new();
        expected_hasher.update(v);

        finalized_sha_to_digest(&expected_hasher.finalize(), v.len() as u64)
    }
}

#[cfg(test)]
mod tests {

    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_from_file() -> Result<(), Box<dyn std::error::Error>> {
        let seed_value = "foobar_baz";
        let seed_value_bytes = seed_value.as_bytes();
        let byte_len = seed_value_bytes.len();

        let mut named_temp_file = NamedTempFile::new()?;

        for _run_idx in 0..100 {
            let ret = named_temp_file.write(seed_value_bytes)?;
            if ret != byte_len {
                panic!("Didn't write everything to file");
            }
        }
        named_temp_file.flush()?;

        let generated: execution::Digest = named_temp_file.path().async_make_digest_from().await?;

        assert_eq!(
            generated.hash.as_str(),
            "bf6ddc41f301ecaa76f2278e245f1f9c24ae67fe3013a68f8fa669c15f9ee941"
        );

        Ok(())
    }
}
