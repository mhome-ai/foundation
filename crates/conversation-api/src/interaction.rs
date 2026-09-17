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

impl Default for InteractionPreview {
    fn default() -> Self {
        Self::confirmation(
            "interaction",
            "Review action",
            "Review this action before continuing.",
        )
    }
}

impl InteractionPreview {
    #[must_use]
    pub fn confirmation(
        action_code: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            action_code: action_code.into(),
            title: title.into(),
            message: message.into(),
            tone: InteractionTone::Normal,
            details: Vec::new(),
            proceed_label: "Continue".to_owned(),
            reject_label: "Cancel".to_owned(),
            client_task: None,
        }
    }

    /// Returns whether the facade supplied a complete display-only interaction contract.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.action_code.trim().is_empty()
            && !self.title.trim().is_empty()
            && !self.proceed_label.trim().is_empty()
            && !self.reject_label.trim().is_empty()
            && self.details.iter().all(|detail| {
                !detail.label.trim().is_empty()
                    && match &detail.value {
                        PreviewValue::Number { value } => value.is_finite(),
                        PreviewValue::TextList { items } => {
                            items.iter().all(|item| !item.trim().is_empty())
                        }
                        PreviewValue::Text { .. } | PreviewValue::Bool { .. } => true,
                    }
            })
            && match &self.client_task {
                Some(ClientTask::InstallIntegrations { integration_ids }) => {
                    !integration_ids.is_empty()
                        && integration_ids
                            .iter()
                            .all(|integration_id| !integration_id.trim().is_empty())
                }
                None => true,
            }
    }
}
