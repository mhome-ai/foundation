//! Bounded latest-state invalidation. Consumers reread authoritative reports;
//! these cursors are process-local and are never persisted as an event queue.
use serde::{Deserialize, Serialize};

pub const HOST_CHANGES_WAIT_PATH: &str = "/v1/changes/wait";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostChangesCursor {
    pub process_generation: Option<String>,
    pub revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostChanges {
    pub host_id: String,
    pub cursor: HostChangesCursor,
}
