use crate::{ConversationQueue, LiveSnapshot, ThreadSnapshot, ThreadSummary};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSession {
    pub activity_at_unix_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ThreadSync {
    Unchanged {
        snapshot_version: u64,
    },
    Delta {
        base_snapshot_version: u64,
        snapshot_version: u64,
        events: Vec<crate::SnapshotUpdatedEvent>,
    },
    Snapshot {
        snapshot: ThreadSnapshot,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadLoadResponse {
    pub thread: ThreadSummary,
    pub session: ThreadSession,
    pub sync: ThreadSync,
    pub live: LiveSnapshot,
    pub queue: ConversationQueue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MessageEnqueueResponse {
    pub request_id: String,
    pub disposition: String,
    pub queue_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InteractionSubmitResponse {
    pub request_id: String,
    pub disposition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequestCancelResponse {
    pub request_id: String,
    pub phase: String,
    pub outcome: String,
    pub queue_version: u64,
}
