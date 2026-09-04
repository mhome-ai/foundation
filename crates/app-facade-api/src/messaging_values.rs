use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAudience {
    Personal,
    Shared,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingAddress {
    pub provider: String,
    pub account_id: String,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lane_id: Option<String>,
    pub audience: ConversationAudience,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalActor {
    pub provider: String,
    pub account_id: String,
    pub external_user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingValueError(&'static str);

impl fmt::Display for MessagingValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for MessagingValueError {}

impl MessagingAddress {
    pub fn new(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        conversation_id: impl Into<String>,
        lane_id: Option<String>,
        audience: ConversationAudience,
    ) -> Result<Self, MessagingValueError> {
        Ok(Self {
            provider: provider_id(provider.into())?,
            account_id: segment(account_id.into(), "messaging account id is invalid")?,
            conversation_id: segment(
                conversation_id.into(),
                "messaging conversation id is invalid",
            )?,
            lane_id: lane_id
                .map(|value| segment(value, "messaging lane id is invalid"))
                .transpose()?,
            audience,
        })
    }
}

impl ExternalActor {
    pub fn new(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        external_user_id: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<Self, MessagingValueError> {
        Ok(Self {
            provider: provider_id(provider.into())?,
            account_id: segment(account_id.into(), "messaging actor account id is invalid")?,
            external_user_id: segment(
                external_user_id.into(),
                "messaging external user id is invalid",
            )?,
            display_name: display_name
                .map(|value| segment(value, "messaging display name is invalid"))
                .transpose()?,
        })
    }
}

fn provider_id(value: String) -> Result<String, MessagingValueError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(MessagingValueError("messaging provider id is invalid"));
    }
    Ok(normalized)
}

fn segment(value: String, error: &'static str) -> Result<String, MessagingValueError> {
    if value.is_empty() || value.trim() != value {
        return Err(MessagingValueError(error));
    }
    Ok(value)
}
