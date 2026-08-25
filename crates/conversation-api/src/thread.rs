use crate::{ConversationMessage, PendingInteraction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadState {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadArchiveReason {
    IdleTimeout,
    UserRequested,
    Replaced,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSummary {
    pub thread_id: String,
    pub title: String,
    pub state: ThreadState,
    pub created_at: String,
    pub updated_at: String,
    pub archived_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_reason: Option<ThreadArchiveReason>,
    pub purge_after: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadCatalog {
    pub catalog_version: u64,
    #[serde(default)]
    pub active_thread_id: Option<String>,
    #[serde(default)]
    pub threads: Vec<ThreadSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QueuedMessage {
    pub request_id: String,
    pub thread_id: String,
    pub content: crate::MessageContent,
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ConversationQueue {
    pub queue_version: u64,
    #[serde(default)]
    pub items: Vec<QueuedMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRun {
    pub request_id: String,
    pub started_at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Completed,
    InteractionRequired,
    Interrupted,
    AgentFailed,
    SystemFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureSource {
    LlmProvider,
    AgentRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunFailure {
    pub source: FailureSource,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RunOutcome {
    pub request_id: String,
    pub status: RunStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<RunFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ThreadSnapshot {
    pub version: u64,
    pub messages: Vec<ConversationMessage>,
    pub active_run: Option<ActiveRun>,
    pub pending_interaction: Option<PendingInteraction>,
    pub run_outcomes: Vec<RunOutcome>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LiveSnapshot {
    pub base_snapshot_version: u64,
    pub last_offset: u64,
    #[serde(default)]
    pub events: Vec<crate::LiveConversationEvent>,
}
