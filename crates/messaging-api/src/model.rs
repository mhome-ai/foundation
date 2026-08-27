use conversation_api::ConversationSurface;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const NORMALIZED_INBOUND_SCHEMA_VERSION: u16 = 1;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NormalizedInboundContent {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_message_id: Option<String>,
    },
    Audio {
        provider_message_id: String,
        provider_file_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_seconds: Option<u32>,
    },
    Interaction {
        action_id: String,
        token: String,
        decision: InteractionDecision,
    },
    InteractionChoice {
        decision: InteractionDecision,
    },
    Selection {
        action_id: String,
        selection_id: String,
        option_index: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedInbound {
    pub schema_version: u16,
    pub event_id: String,
    pub address: MessagingAddress,
    pub actor: ExternalActor,
    pub content: NormalizedInboundContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at_ms: Option<i64>,
}

impl NormalizedInbound {
    pub fn validate(self) -> Result<Self, MessagingModelError> {
        if self.schema_version != NORMALIZED_INBOUND_SCHEMA_VERSION {
            return Err(MessagingModelError(
                "unsupported normalized messaging schema version",
            ));
        }
        let event_id = segment(self.event_id, "messaging event id is invalid")?;
        let address = self.address.validate()?;
        let actor = self.actor.validate()?;
        if actor.provider != address.provider || actor.account_id != address.account_id {
            return Err(MessagingModelError(
                "messaging actor and address account do not match",
            ));
        }
        validate_content(&self.content)?;
        Ok(Self {
            schema_version: self.schema_version,
            event_id,
            address,
            actor,
            content: self.content,
            occurred_at_ms: self.occurred_at_ms,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboundDispositionKind {
    Ignored,
    ProviderHandled,
    Conversation,
    RetryableFailure,
    TerminalFailure,
}

fn validate_content(content: &NormalizedInboundContent) -> Result<(), MessagingModelError> {
    match content {
        NormalizedInboundContent::Text {
            text,
            provider_message_id,
        } => {
            segment_ref(text, "messaging text is invalid")?;
            optional_ref(provider_message_id, "provider message id is invalid")?;
        }
        NormalizedInboundContent::Audio {
            provider_message_id,
            provider_file_id,
            ..
        } => {
            segment_ref(provider_message_id, "provider message id is invalid")?;
            segment_ref(provider_file_id, "provider file id is invalid")?;
        }
        NormalizedInboundContent::Interaction {
            action_id, token, ..
        } => {
            segment_ref(action_id, "interaction action id is invalid")?;
            segment_ref(token, "interaction token is invalid")?;
        }
        NormalizedInboundContent::InteractionChoice { .. } => {}
        NormalizedInboundContent::Selection {
            action_id,
            selection_id,
            ..
        } => {
            segment_ref(action_id, "selection action id is invalid")?;
            segment_ref(selection_id, "selection id is invalid")?;
        }
    }
    Ok(())
}

fn provider_id(value: String) -> Result<String, MessagingModelError> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty()
        || !normalized
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(MessagingModelError("messaging provider id is invalid"));
    }
    Ok(normalized)
}

fn segment(value: String, error: &'static str) -> Result<String, MessagingModelError> {
    segment_ref(&value, error)?;
    Ok(value)
}

fn optional_segment(value: String) -> Result<String, MessagingModelError> {
    segment(value, "messaging display name is invalid")
}

fn segment_ref(value: &str, error: &'static str) -> Result<(), MessagingModelError> {
    if value.is_empty() || value.trim() != value {
        return Err(MessagingModelError(error));
    }
    Ok(())
}

fn optional_ref(value: &Option<String>, error: &'static str) -> Result<(), MessagingModelError> {
    if let Some(value) = value {
        segment_ref(value, error)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_owns_lane_and_canonical_surface_mapping() {
        let address = MessagingAddress::new(
            "Telegram",
            "bot:1",
            "chat:2",
            Some("topic:3".to_string()),
            ConversationAudience::Shared,
        )
        .unwrap();
        assert_eq!(address.provider, "telegram");
        assert_eq!(address.base_address().lane_id, None);
        let surface = address.conversation_surface().unwrap();
        let route = surface.messaging_route().unwrap();
        assert_eq!(route.lane_id, Some("topic:3"));
        assert!(route.group);
    }

    #[test]
    fn inbound_rejects_actor_from_another_provider_account() {
        let inbound = NormalizedInbound {
            schema_version: NORMALIZED_INBOUND_SCHEMA_VERSION,
            event_id: "event".to_string(),
            address: MessagingAddress::new(
                "telegram",
                "bot-a",
                "chat",
                None,
                ConversationAudience::Personal,
            )
            .unwrap(),
            actor: ExternalActor::new("telegram", "bot-b", "user", None).unwrap(),
            content: NormalizedInboundContent::Text {
                text: "hello".to_string(),
                provider_message_id: None,
            },
            occurred_at_ms: None,
        };
        assert!(inbound.validate().is_err());
    }
}
