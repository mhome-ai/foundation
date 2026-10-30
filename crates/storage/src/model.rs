use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, pin::Pin};
use tokio::io::AsyncRead;

pub type Reader = Pin<Box<dyn AsyncRead + Send + Unpin>>;
pub const BACKEND_DESCRIBE: &str = "/storage/backend/describe";
pub const BACKEND_GRANT: &str = "/storage/backend/grant";
pub const PROTOCOL: &str = "storage.v3";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    pub repository_id: String,
    /// Identifies a filesystem for display; capacity from identical volumes is never summed.
    pub volume_id: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub stop_at_available_bytes: u64,
    pub warning_at_available_bytes: u64,
}
impl Capacity {
    pub fn writable_bytes(&self) -> u64 {
        self.available_bytes
            .saturating_sub(self.stop_at_available_bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Blob {
    pub id: String,
    pub size_bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ReadSource {
    Local {
        path: PathBuf,
    },
    Http {
        url: String,
        token: String,
        host_public_key: String,
    },
}

/// Backends never see Folder names, quotas, Artifact identities, or eviction policy.
#[async_trait]
pub trait Backend: Send + Sync {
    fn repository_id(&self) -> &str;
    async fn capacity(&self) -> anyhow::Result<Capacity>;
    /// Reserve real physical space before the caller consumes its input stream.
    async fn prepare(&self, _owner: &str, _blob: &Blob) -> anyhow::Result<()> {
        Ok(())
    }
    async fn put(&self, owner: &str, blob: &Blob, reader: Reader) -> anyhow::Result<()>;
    async fn stat(&self, owner: &str, id: &str) -> anyhow::Result<Option<Blob>>;
    async fn source(&self, owner: &str, id: &str) -> anyhow::Result<ReadSource>;
    /// Success confirms a durable physical unlink, including an idempotent retry.
    async fn delete(&self, owner: &str, id: &str) -> anyhow::Result<()>;
}

#[async_trait]
pub trait Resolver: Send + Sync {
    /// Only an authenticated, bound, currently reachable repository is returned.
    async fn preferred(
        &self,
        tenant: &str,
        scope: &str,
    ) -> anyhow::Result<Option<(std::sync::Arc<dyn Backend>, String)>>;
    async fn resolve(
        &self,
        tenant: &str,
        scope: &str,
        repository: &str,
        handle: &str,
    ) -> anyhow::Result<std::sync::Arc<dyn Backend>>;
}

#[derive(Clone)]
pub enum Input {
    File(PathBuf),
    Bytes(std::sync::Arc<Vec<u8>>),
    Stream(std::sync::Arc<tokio::sync::Mutex<Option<Reader>>>),
}
impl Input {
    pub async fn reader(&self) -> anyhow::Result<Reader> {
        Ok(match self {
            Self::File(path) => Box::pin(tokio::fs::File::open(path).await?),
            Self::Bytes(bytes) => Box::pin(std::io::Cursor::new(bytes.as_ref().clone())),
            Self::Stream(reader) => reader
                .lock()
                .await
                .take()
                .ok_or_else(|| anyhow::anyhow!("input stream already consumed"))?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Overflow {
    #[default]
    Reject,
    EvictOldest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub name: String,
    pub system: bool,
    pub max_bytes: Option<u64>,
    pub overflow_strategy: Overflow,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub object_count: u64,
    pub evicted_object_count: u64,
    pub evicted_bytes: u64,
    pub rejected_write_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Object {
    pub object_id: String,
    pub folder_id: String,
    pub key: String,
    pub repository_id: String,
    pub owner: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub content_type: Option<String>,
    pub committed_at_ms: u64,
}

#[derive(Debug, Default)]
pub struct Usage {
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub object_count: u64,
    pub folder_count: u64,
    pub active_upload_count: u64,
    pub pending_deletion_count: u64,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
