//! Transport-neutral physical byte repository metadata. No Folder policy.
use serde::{Deserialize, Serialize};
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
