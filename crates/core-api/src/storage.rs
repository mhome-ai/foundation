use serde::{Deserialize, Serialize};
use std::fmt;

pub const STORAGE_NODE_TYPE: &str = "storage";
pub const STORAGE_PROTOCOL_VERSION: &str = "storage.v3";

pub const STORAGE_READINESS_ROUTE: &str = "/storage/readiness";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageOverflowStrategy {
    #[default]
    Reject,
    EvictOldest,
}

impl StorageOverflowStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::EvictOldest => "evictOldest",
        }
    }

    pub fn parse(value: &str) -> Result<Self, UnsupportedStorageOverflowStrategy> {
        match value {
            "reject" => Ok(Self::Reject),
            "evictOldest" => Ok(Self::EvictOldest),
            _ => Err(UnsupportedStorageOverflowStrategy(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedStorageOverflowStrategy(String);

impl fmt::Display for UnsupportedStorageOverflowStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported storage overflow strategy: {}",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedStorageOverflowStrategy {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoragePermission {
    Read,
    Write,
    Delete,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageObject {
    pub key: String,
    pub object_id: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub content_type: Option<String>,
    pub committed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageObjectList {
    pub objects: Vec<StorageObject>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageWriteState {
    Writable,
    LowSpace,
    BlockedByPhysicalSpace,
}
