//! One-shot human interactions. Business cards and response validation belong to their owners.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionTone {
    #[default]
    Normal,
    Danger,
}

/// A registered presentation type and its structured, business-owned data.
/// `card_type` selects trusted client code; it is never a route, script or callback.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionCard {
    #[serde(rename = "type")]
    pub card_type: String,
    pub title: String,
    pub message: String,
    pub tone: InteractionTone,
    pub continue_label: String,
    pub cancel_label: String,
    pub data: Map<String, Value>,
}

impl InteractionCard {
    #[must_use]
    pub fn new(
        card_type: impl Into<String>,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            card_type: card_type.into(),
            title: title.into(),
            message: message.into(),
            tone: InteractionTone::Normal,
            continue_label: "Continue".into(),
            cancel_label: "Cancel".into(),
            data: Map::new(),
        }
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.card_type.is_empty()
            && self
                .card_type
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'.' || c == b'_')
            && !self.title.trim().is_empty()
            && !self.continue_label.trim().is_empty()
            && !self.cancel_label.trim().is_empty()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    Approval,
    UserAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingInteractionItem {
    pub action_id: String,
    pub kind: InteractionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
    pub card: InteractionCard,
}

impl PendingInteractionItem {
    #[must_use]
    pub fn accepts(&self, response: &InteractionResponse) -> bool {
        self.action_id == response.action_id
            && response
                .client_report
                .as_ref()
                .is_none_or(|report| self.kind == InteractionKind::UserAction && report.is_valid())
            && matches!(
                (self.kind, &response.response),
                (
                    InteractionKind::Approval,
                    InteractionAnswer::Approve | InteractionAnswer::Reject
                ) | (
                    InteractionKind::UserAction,
                    InteractionAnswer::Continue { .. } | InteractionAnswer::Cancel
                )
            )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingInteraction {
    pub request_id: String,
    pub batch_id: String,
    pub title: String,
    pub message: String,
    pub items: Vec<PendingInteractionItem>,
    /// Persisted answers. Clients display the first item not present here.
    /// Answer bodies remain private until the tool produces its final result.
    pub answered_action_ids: Vec<String>,
}

/// A response consumes one interaction. It never requests another waiting phase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum InteractionAnswer {
    #[serde(deserialize_with = "crate::execution::deserialize_empty_variant")]
    Approve,
    #[serde(deserialize_with = "crate::execution::deserialize_empty_variant")]
    Reject,
    Continue {
        data: Value,
    },
    #[serde(deserialize_with = "crate::execution::deserialize_empty_variant")]
    Cancel,
}

/// Client observations from one Operation, never evidence that its business work succeeded.
/// Only safe public warning text is accepted; raw Facade details and credentials are excluded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientOperationWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ClientOperationReport {
    pub warnings: Vec<ClientOperationWarning>,
}

impl ClientOperationReport {
    pub fn is_valid(&self) -> bool {
        !self.warnings.is_empty()
            && self.warnings.iter().all(|warning| {
                !warning.code.trim().is_empty() && !warning.message.trim().is_empty()
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionResponse {
    pub action_id: String,
    pub response: InteractionAnswer,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_report: Option<ClientOperationReport>,
}
