//! Frozen request envelope for Pod-only endpoints. Not accepted by `/app/*`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use crate::ActionMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Json,
    Text,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PodControl {
    pub mode: ActionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepared_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl PodControl {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self.mode {
            ActionMode::Direct | ActionMode::Prepare => {
                if self.prepared_action_id.is_some() || self.approval_token.is_some() {
                    return Err("direct and prepare controls cannot carry commit credentials");
                }
            }
            ActionMode::Commit => {
                if self
                    .prepared_action_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                    || self
                        .approval_token
                        .as_deref()
                        .is_none_or(|value| value.trim().is_empty())
                {
                    return Err("commit control requires preparedActionId and approvalToken");
                }
            }
            ActionMode::Reject => {
                if self
                    .prepared_action_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
                {
                    return Err("reject control requires preparedActionId");
                }
                if self.approval_token.is_some() {
                    return Err("reject control cannot carry approvalToken");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PodCall {
    pub control: PodControl,
    pub input: Value,
}

impl PodCall {
    #[must_use]
    pub fn direct(input: Value) -> Self {
        Self {
            control: PodControl::default(),
            input,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.control.validate()
    }
}
