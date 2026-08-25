use crate::{
    ActiveRun, ConversationMessage, PendingInteraction, QueuedMessage, RunOutcome, ThreadSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

macro_rules! event_type {
    ($name:ident, $variant:ident, $wire:literal) => {
        #[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
        pub enum $name {
            #[default]
            #[serde(rename = $wire)]
            $variant,
        }
    };
}

event_type!(
    SnapshotUpdatedEventType,
    SnapshotUpdated,
    "snapshot.updated"
);
event_type!(
    CatalogChangedEventType,
    CatalogChanged,
    "thread.catalog_changed"
);
event_type!(QueueChangedEventType, QueueChanged, "queue.changed");
event_type!(
    SessionPolicyChangedEventType,
    SessionPolicyChanged,
    "session.policy_changed"
);
event_type!(
    AssistantPreviewEventType,
    AssistantPreview,
    "assistant.preview"
);
event_type!(RunProgressEventType, RunProgress, "run.progress");
event_type!(
    RunSystemFailedEventType,
    RunSystemFailed,
    "run.system_failed"
);
event_type!(DebugEventType, RunDebug, "run.debug");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotUpdatedEvent {
    #[serde(rename = "type")]
    pub event_type: SnapshotUpdatedEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub snapshot_version: u64,
    pub occurred_at: String,
    pub data: SnapshotUpdatedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SnapshotUpdatedData {
    #[serde(default)]
    pub session_activity_at_unix_ms: i64,
    #[serde(default)]
    pub messages_added: Vec<ConversationMessage>,
    pub active_run: Option<ActiveRun>,
    pub pending_interaction: Option<PendingInteraction>,
    pub run_outcome: Option<RunOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueChangedEvent {
    #[serde(rename = "type")]
    pub event_type: QueueChangedEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub queue_version: u64,
    pub occurred_at: String,
    pub data: QueueChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueChangedData {
    pub items: Vec<QueuedMessage>,
    pub active_run: Option<ActiveRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogChangedEvent {
    #[serde(rename = "type")]
    pub event_type: CatalogChangedEventType,
    pub surface_id: String,
    pub catalog_version: u64,
    pub occurred_at: String,
    pub data: CatalogChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CatalogChangedData {
    pub active_thread_id: Option<String>,
    pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionPolicyChangedEvent {
    #[serde(rename = "type")]
    pub event_type: SessionPolicyChangedEventType,
    pub version: u64,
    pub surface_id: String,
    pub occurred_at: String,
    pub data: SessionPolicyChangedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionPolicyChangedData {
    pub idle_timeout_minutes: u32,
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistantPreviewEvent {
    #[serde(rename = "type")]
    pub event_type: AssistantPreviewEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub base_snapshot_version: u64,
    pub offset: u64,
    pub occurred_at: String,
    pub data: AssistantPreviewData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssistantPreviewData {
    pub text: String,
    pub append: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunProgressEvent {
    #[serde(rename = "type")]
    pub event_type: RunProgressEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub base_snapshot_version: u64,
    pub offset: u64,
    pub occurred_at: String,
    pub data: RunProgressData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RunProgressData {
    Thinking {},
    Planning {},
    WaitingExternal {},
    ToolScheduled { tool_names: Vec<String> },
    ToolStarted { tool_name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunSystemFailedEvent {
    #[serde(rename = "type")]
    pub event_type: RunSystemFailedEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub base_snapshot_version: u64,
    pub offset: u64,
    pub occurred_at: String,
    pub data: RunSystemFailedData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunSystemFailedData {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum LiveConversationEvent {
    AssistantPreview(AssistantPreviewEvent),
    RunProgress(RunProgressEvent),
    RunSystemFailed(RunSystemFailedEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveEventMetadata {
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub base_snapshot_version: u64,
    pub offset: u64,
    pub occurred_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveConversationEventData {
    AssistantPreview(AssistantPreviewData),
    RunProgress(RunProgressData),
    RunSystemFailed(RunSystemFailedData),
}

impl LiveConversationEvent {
    pub fn new(metadata: LiveEventMetadata, data: LiveConversationEventData) -> Self {
        match data {
            LiveConversationEventData::AssistantPreview(data) => {
                Self::AssistantPreview(AssistantPreviewEvent {
                    event_type: AssistantPreviewEventType::default(),
                    surface_id: metadata.surface_id,
                    thread_id: metadata.thread_id,
                    request_id: metadata.request_id,
                    base_snapshot_version: metadata.base_snapshot_version,
                    offset: metadata.offset,
                    occurred_at: metadata.occurred_at,
                    data,
                })
            }
            LiveConversationEventData::RunProgress(data) => Self::RunProgress(RunProgressEvent {
                event_type: RunProgressEventType::default(),
                surface_id: metadata.surface_id,
                thread_id: metadata.thread_id,
                request_id: metadata.request_id,
                base_snapshot_version: metadata.base_snapshot_version,
                offset: metadata.offset,
                occurred_at: metadata.occurred_at,
                data,
            }),
            LiveConversationEventData::RunSystemFailed(data) => {
                Self::RunSystemFailed(RunSystemFailedEvent {
                    event_type: RunSystemFailedEventType::default(),
                    surface_id: metadata.surface_id,
                    thread_id: metadata.thread_id,
                    request_id: metadata.request_id,
                    base_snapshot_version: metadata.base_snapshot_version,
                    offset: metadata.offset,
                    occurred_at: metadata.occurred_at,
                    data,
                })
            }
        }
    }

    pub fn surface_id(&self) -> &str {
        match self {
            Self::AssistantPreview(event) => &event.surface_id,
            Self::RunProgress(event) => &event.surface_id,
            Self::RunSystemFailed(event) => &event.surface_id,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::AssistantPreview(_) => "assistant.preview",
            Self::RunProgress(_) => "run.progress",
            Self::RunSystemFailed(_) => "run.system_failed",
        }
    }

    pub fn base_snapshot_version(&self) -> u64 {
        match self {
            Self::AssistantPreview(event) => event.base_snapshot_version,
            Self::RunProgress(event) => event.base_snapshot_version,
            Self::RunSystemFailed(event) => event.base_snapshot_version,
        }
    }

    pub fn offset(&self) -> u64 {
        match self {
            Self::AssistantPreview(event) => event.offset,
            Self::RunProgress(event) => event.offset,
            Self::RunSystemFailed(event) => event.offset,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[allow(clippy::large_enum_variant)]
pub enum ConversationEvent {
    SnapshotUpdated(SnapshotUpdatedEvent),
    CatalogChanged(CatalogChangedEvent),
    QueueChanged(QueueChangedEvent),
    SessionPolicyChanged(SessionPolicyChangedEvent),
    AssistantPreview(AssistantPreviewEvent),
    RunProgress(RunProgressEvent),
    RunSystemFailed(RunSystemFailedEvent),
}

impl ConversationEvent {
    pub fn surface_id(&self) -> &str {
        match self {
            Self::SnapshotUpdated(event) => &event.surface_id,
            Self::CatalogChanged(event) => &event.surface_id,
            Self::QueueChanged(event) => &event.surface_id,
            Self::SessionPolicyChanged(event) => &event.surface_id,
            Self::AssistantPreview(event) => &event.surface_id,
            Self::RunProgress(event) => &event.surface_id,
            Self::RunSystemFailed(event) => &event.surface_id,
        }
    }

    pub fn event_type(&self) -> &'static str {
        match self {
            Self::SnapshotUpdated(_) => "snapshot.updated",
            Self::CatalogChanged(_) => "thread.catalog_changed",
            Self::QueueChanged(_) => "queue.changed",
            Self::SessionPolicyChanged(_) => "session.policy_changed",
            Self::AssistantPreview(_) => "assistant.preview",
            Self::RunProgress(_) => "run.progress",
            Self::RunSystemFailed(_) => "run.system_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DebugEvent {
    #[serde(rename = "type")]
    pub event_type: DebugEventType,
    pub surface_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub occurred_at: String,
    pub data: DebugEventData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct DebugEventData {
    pub scope: String,
    pub payload: Value,
}
