use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionMode {
    #[default]
    Direct,
    Prepare,
    Commit,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    Json,
    Text,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FacadeControl {
    pub mode: ActionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepared_action_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approval_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
}

impl FacadeControl {
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
pub struct FacadeCall {
    pub control: FacadeControl,
    pub input: Value,
}

impl FacadeCall {
    #[must_use]
    pub fn direct(input: Value) -> Self {
        Self {
            control: FacadeControl::default(),
            input,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        self.control.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_call_has_a_strict_envelope() {
        let call: FacadeCall = serde_json::from_value(serde_json::json!({
            "control": {"mode": "direct"},
            "input": {"value": 1}
        }))
        .unwrap();
        assert_eq!(call.control.mode, ActionMode::Direct);
        assert_eq!(call.input["value"], 1);
        assert!(serde_json::from_value::<FacadeCall>(serde_json::json!({
            "mode": "direct",
            "input": {}
        }))
        .is_err());
    }

    #[test]
    fn action_credentials_are_mode_specific() {
        let invalid = FacadeControl {
            mode: ActionMode::Commit,
            prepared_action_id: Some("action-1".into()),
            ..FacadeControl::default()
        };
        assert!(invalid.validate().is_err());

        let whitespace = FacadeControl {
            mode: ActionMode::Reject,
            prepared_action_id: Some(" \t".into()),
            ..FacadeControl::default()
        };
        assert!(whitespace.validate().is_err());
    }
}
