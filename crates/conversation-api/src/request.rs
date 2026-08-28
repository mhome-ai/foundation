use crate::{InteractionDecision, MessageContent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThreadListRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreadCreateRequest {
    pub operation_id: String,
    pub expected_catalog_version: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreadArchiveRequest {
    pub thread_id: String,
    pub expected_catalog_version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreadRotateRequest {
    pub operation_id: String,
    pub thread_id: String,
    pub expected_catalog_version: u64,
    pub expected_snapshot_version: u64,
    pub expected_setting_version: u64,
    pub reason: ThreadRotateReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThreadRotateReason {
    IdleTimeout,
    UserRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThreadLoadRequest {
    pub thread_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_snapshot_version: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessageEnqueueRequest {
    pub thread_id: String,
    pub request_id: String,
    pub content: MessageContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_mode: Option<ConversationAccessMode>,
    pub agent_model_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub allow_tools: bool,
    pub debug: bool,
}

/// Submits one user turn without requiring the caller to select or manage a thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TurnSubmitRequest {
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub content: MessageContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_mode: Option<ConversationAccessMode>,
    pub agent_model_mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub allow_tools: bool,
    pub debug: bool,
    pub occurred_at_unix_ms: i64,
}

/// Starts a new active session without exposing thread storage preconditions to the caller.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionStartRequest {
    pub operation_id: String,
    pub reason: SessionStartReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionStartReason {
    UserRequested,
}

/// Answers the current pending interaction without requiring thread or policy state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionAnswerRequest {
    pub request_id: String,
    pub batch_id: String,
    pub decisions: Vec<InteractionDecision>,
    pub occurred_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAccessMode {
    Interactive,
    FullAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QueueReorderRequest {
    pub thread_id: String,
    pub expected_queue_version: u64,
    pub ordered_request_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestCancelRequest {
    pub thread_id: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionSubmitRequest {
    pub thread_id: String,
    pub batch_id: String,
    pub decisions: Vec<InteractionDecision>,
}
