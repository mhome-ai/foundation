use crate::{
    ActiveRun, ConversationMessage, PendingInteraction, QueuedMessage, RunOutcome, ThreadSummary,
};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
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

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ConversationEvent {
    SnapshotUpdated(SnapshotUpdatedEvent),
    CatalogChanged(CatalogChangedEvent),
    QueueChanged(QueueChangedEvent),
    SessionPolicyChanged(SessionPolicyChangedEvent),
    Live(crate::LiveEvent),
}

impl<'de> Deserialize<'de> for ConversationEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let event_type = value
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| D::Error::custom("conversation event type is required"))?;
        match event_type {
            "snapshot.updated" => serde_json::from_value(value)
                .map(Self::SnapshotUpdated)
                .map_err(D::Error::custom),
            "thread.catalog_changed" => serde_json::from_value(value)
                .map(Self::CatalogChanged)
                .map_err(D::Error::custom),
            "queue.changed" => serde_json::from_value(value)
                .map(Self::QueueChanged)
                .map_err(D::Error::custom),
            "session.policy_changed" => serde_json::from_value(value)
                .map(Self::SessionPolicyChanged)
                .map_err(D::Error::custom),
            _ => serde_json::from_value(value)
                .map(Self::Live)
                .map_err(D::Error::custom),
        }
    }
}

impl ConversationEvent {
    pub fn surface_id(&self) -> &str {
        match self {
            Self::SnapshotUpdated(event) => &event.surface_id,
            Self::CatalogChanged(event) => &event.surface_id,
            Self::QueueChanged(event) => &event.surface_id,
            Self::SessionPolicyChanged(event) => &event.surface_id,
            Self::Live(event) => &event.surface_id,
        }
    }

    pub fn event_type(&self) -> &str {
        match self {
            Self::SnapshotUpdated(event) => &event.event_type,
            Self::CatalogChanged(event) => &event.event_type,
            Self::QueueChanged(event) => &event.event_type,
            Self::SessionPolicyChanged(event) => &event.event_type,
            Self::Live(event) => &event.event_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DebugEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub occurred_at: String,
    pub data: DebugEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DebugEventData {
    pub scope: String,
    pub payload: Value,
}
