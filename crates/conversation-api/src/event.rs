use crate::{
    ActiveRun, ConversationMessage, PendingInteraction, QueuedMessage, RunOutcome, ThreadSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotUpdatedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub snapshot_version: u64,
    pub occurred_at: String,
    pub data: SnapshotUpdatedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotUpdatedData {
    pub reason: String,
    #[serde(default)]
    pub session_activity_at_unix_ms: i64,
    #[serde(default)]
    pub messages_added: Vec<ConversationMessage>,
    pub active_run: Option<ActiveRun>,
    pub pending_interaction: Option<PendingInteraction>,
    pub run_outcome: Option<RunOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueueChangedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub surface_id: String,
    pub thread_id: String,
    pub queue_version: u64,
    pub occurred_at: String,
    pub data: QueueChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueueChangedData {
    pub items: Vec<QueuedMessage>,
    pub active_run: Option<ActiveRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogChangedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub surface_id: String,
    pub catalog_version: u64,
    pub occurred_at: String,
    pub data: CatalogChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogChangedData {
    pub active_thread_id: Option<String>,
    pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionPolicyChangedEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub version: u64,
    pub surface_id: String,
    pub occurred_at: String,
    pub data: SessionPolicyChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionPolicyChangedData {
    pub idle_timeout_minutes: u32,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ConversationEvent {
    #[serde(rename = "snapshot.updated")]
    SnapshotUpdated {
        #[serde(default)]
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "snapshotVersion")]
        snapshot_version: u64,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: Box<SnapshotUpdatedData>,
    },
    #[serde(rename = "thread.catalog_changed")]
    CatalogChanged {
        #[serde(default)]
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "catalogVersion")]
        catalog_version: u64,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: CatalogChangedData,
    },
    #[serde(rename = "queue.changed")]
    QueueChanged {
        #[serde(default)]
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        #[serde(rename = "queueVersion")]
        queue_version: u64,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: QueueChangedData,
    },
    #[serde(rename = "session.policy_changed")]
    SessionPolicyChanged {
        version: u64,
        #[serde(default)]
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: SessionPolicyChangedData,
    },
    #[serde(rename = "run.progress")]
    Live {
        #[serde(default)]
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        #[serde(rename = "requestId")]
        request_id: String,
        #[serde(rename = "baseSnapshotVersion")]
        base_snapshot_version: u64,
        offset: u64,
        #[serde(rename = "occurredAt")]
        occurred_at: String,
        data: Value,
    },
}

impl ConversationEvent {
    pub fn surface_id(&self) -> &str {
        match self {
            Self::SnapshotUpdated { surface_id, .. }
            | Self::CatalogChanged { surface_id, .. }
            | Self::QueueChanged { surface_id, .. }
            | Self::SessionPolicyChanged { surface_id, .. }
            | Self::Live { surface_id, .. } => surface_id,
        }
    }
}
