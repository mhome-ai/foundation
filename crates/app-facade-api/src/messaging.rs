#[path = "control.rs"]
mod control;
#[path = "targets.rs"]
mod targets;
pub use control::*;
use conversation_api::ConversationSurface;
use serde::{Deserialize, Serialize};
use std::fmt;
pub use targets::*;

// App Facade owns its public messaging identity types. Their wire shapes intentionally overlap
// Core's internal messaging model, but neither package depends on the other.
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessagingModelError(&'static str);

impl fmt::Display for MessagingModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for MessagingModelError {}

impl MessagingAddress {
    pub fn new(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        conversation_id: impl Into<String>,
        lane_id: Option<String>,
        audience: ConversationAudience,
    ) -> Result<Self, MessagingModelError> {
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

    #[must_use]
    pub fn base_address(&self) -> Self {
        let mut base = self.clone();
        base.lane_id = None;
        base
    }

    pub fn conversation_surface(&self) -> Result<ConversationSurface, MessagingModelError> {
        let result = match self.audience {
            ConversationAudience::Personal => ConversationSurface::messaging_personal(
                &self.provider,
                &self.account_id,
                &self.conversation_id,
                self.lane_id.clone(),
            ),
            ConversationAudience::Shared => ConversationSurface::messaging_group(
                &self.provider,
                &self.account_id,
                &self.conversation_id,
                self.lane_id.clone(),
            ),
        };
        result.map_err(|_| MessagingModelError("messaging address cannot form a surface"))
    }

    pub fn validate(self) -> Result<Self, MessagingModelError> {
        Self::new(
            self.provider,
            self.account_id,
            self.conversation_id,
            self.lane_id,
            self.audience,
        )
    }
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

impl ExternalActor {
    pub fn new(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        external_user_id: impl Into<String>,
        display_name: Option<String>,
    ) -> Result<Self, MessagingModelError> {
        Ok(Self {
            provider: provider_id(provider.into())?,
            account_id: segment(account_id.into(), "messaging actor account id is invalid")?,
            external_user_id: segment(
                external_user_id.into(),
                "messaging external user id is invalid",
            )?,
            display_name: display_name.map(optional_segment).transpose()?,
        })
    }

    pub fn validate(self) -> Result<Self, MessagingModelError> {
        Self::new(
            self.provider,
            self.account_id,
            self.external_user_id,
            self.display_name,
        )
    }
}

fn provider_id(value: String) -> Result<String, MessagingModelError> {
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
        return Err(MessagingModelError("messaging provider id is invalid"));
    }
    Ok(normalized)
}

fn segment(value: String, error: &'static str) -> Result<String, MessagingModelError> {
    if value.is_empty() || value.trim() != value {
        return Err(MessagingModelError(error));
    }
    Ok(value)
}

fn optional_segment(value: String) -> Result<String, MessagingModelError> {
    segment(value, "messaging display name is invalid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_messaging_identity_is_validated_without_core_api() {
        let address = MessagingAddress::new(
            "Telegram",
            "bot:1",
            "chat:2",
            Some("topic:3".to_string()),
            ConversationAudience::Shared,
        )
        .unwrap();
        assert_eq!(address.provider, "telegram");
        assert!(address.conversation_surface().is_ok());
        assert_eq!(address.base_address().lane_id, None);
        assert!(ExternalActor::new("telegram", " bot", "user", None).is_err());
    }
}
