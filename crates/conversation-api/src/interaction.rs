use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PreviewValue {
    Text { text: String },
    TextList { items: Vec<String> },
    Number { value: f64 },
    Bool { value: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PreviewDetail {
    pub label: String,
    pub value: PreviewValue,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionTone {
    #[default]
    Normal,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum ClientTask {
    InstallIntegrations { integration_ids: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InteractionPreview {
    pub action_code: String,
    pub title: String,
    pub message: String,
    #[serde(default)]
    pub tone: InteractionTone,
    #[serde(default)]
    pub details: Vec<PreviewDetail>,
    pub proceed_label: String,
    pub reject_label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_task: Option<ClientTask>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    #[default]
    Approval,
    UserAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingInteractionItem {
    pub action_id: String,
    pub kind: InteractionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub preview: InteractionPreview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PendingInteraction {
    pub request_id: String,
    pub batch_id: String,
    pub title: String,
    pub message: String,
    pub items: Vec<PendingInteractionItem>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionDecisionValue {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionDecision {
    pub action_id: String,
    pub decision: InteractionDecisionValue,
}
