use conversation_api::ConversationSurface;
use serde::{Deserialize, Serialize};
use std::fmt;

pub const NORMALIZED_INBOUND_SCHEMA_VERSION: u16 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAudience {
    Personal,
    Shared,
}

/// Whether a provider could determine that a message explicitly targets the bot.
///
/// Shared-conversation policy drops `Unaddressed` messages. `Unknown` remains
/// admissible for providers that cannot expose an equivalent signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageAttention {
    Addressed,
    Unaddressed,
    Unknown,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionOption {
    pub label: String,
    pub token: String,
}

impl ActionOption {
    pub fn new(
        label: impl Into<String>,
        token: impl Into<String>,
    ) -> Result<Self, MessagingModelError> {
        Ok(Self {
            label: segment(label.into(), "action label is invalid")?,
            token: segment(token.into(), "action token is invalid")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActionSet {
    pub options: Vec<ActionOption>,
}

impl ActionSet {
    pub fn new(options: Vec<ActionOption>) -> Result<Self, MessagingModelError> {
        if options.is_empty() {
            return Err(MessagingModelError("action set cannot be empty"));
        }
        for option in &options {
            segment_ref(&option.label, "action label is invalid")?;
            segment_ref(&option.token, "action token is invalid")?;
        }
        Ok(Self { options })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderMediaKind {
    Image,
    Audio,
    Video,
    File,
}

/// Provider-scoped, short-lived media handle. It must be materialized before entering Conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderMediaRef {
    pub handle: String,
    pub kind: ProviderMediaKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_px: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_px: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transcript: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum InboundMessagePart {
    Text { text: String },
    Media { reference: ProviderMediaRef },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum NormalizedInboundContent {
    Message {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        provider_message_id: Option<String>,
        attention: MessageAttention,
        parts: Vec<InboundMessagePart>,
    },
    ActionSelected {
        token: String,
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
    pub conversation_display_name: Option<String>,
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
        if self.occurred_at_ms.is_some_and(|value| value < 0) {
            return Err(MessagingModelError(
                "messaging occurrence time cannot be negative",
            ));
        }
        validate_content(&self.content)?;
        optional_ref(
            &self.conversation_display_name,
            "messaging conversation display name is invalid",
        )?;
        Ok(Self {
            schema_version: self.schema_version,
            event_id,
            address,
            actor,
            content: self.content,
            conversation_display_name: self.conversation_display_name,
            occurred_at_ms: self.occurred_at_ms,
        })
    }
}

fn validate_content(content: &NormalizedInboundContent) -> Result<(), MessagingModelError> {
    match content {
        NormalizedInboundContent::Message {
            provider_message_id,
            attention: _,
            parts,
        } => {
            if parts.is_empty() || parts.len() > 16 {
                return Err(MessagingModelError(
                    "messaging message part count is invalid",
                ));
            }
            optional_ref(provider_message_id, "provider message id is invalid")?;
            for part in parts {
                match part {
                    InboundMessagePart::Text { text } if text.is_empty() => {
                        return Err(MessagingModelError("messaging text is invalid"));
                    }
                    InboundMessagePart::Text { .. } => {}
                    InboundMessagePart::Media { reference } => validate_media(reference)?,
                }
            }
        }
        NormalizedInboundContent::ActionSelected { token } => {
            segment_ref(token, "action token is invalid")?;
        }
    }
    Ok(())
}

fn validate_media(reference: &ProviderMediaRef) -> Result<(), MessagingModelError> {
    segment_ref(&reference.handle, "provider media handle is invalid")?;
    optional_ref(&reference.mime_type, "provider media MIME type is invalid")?;
    optional_ref(&reference.file_name, "provider media file name is invalid")?;
    optional_ref(&reference.caption, "provider media caption is invalid")?;
    optional_ref(
        &reference.transcript,
        "provider media transcript is invalid",
    )?;
    if reference.size_bytes == Some(0)
        || reference.duration_ms == Some(0)
        || reference.width_px == Some(0)
        || reference.height_px == Some(0)
    {
        return Err(MessagingModelError("provider media metadata is invalid"));
    }
    if reference.width_px.is_some() != reference.height_px.is_some() {
        return Err(MessagingModelError(
            "provider media dimensions must be complete",
        ));
    }
    Ok(())
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
            content: NormalizedInboundContent::Message {
                provider_message_id: None,
                attention: MessageAttention::Unknown,
                parts: vec![InboundMessagePart::Text {
                    text: "hello".to_string(),
                }],
            },
            conversation_display_name: None,
            occurred_at_ms: None,
        };
        assert!(inbound.validate().is_err());
    }

    #[test]
    fn actions_only_expose_labels_and_opaque_tokens() {
        let actions = ActionSet::new(vec![
            ActionOption::new("Approve", "route-approve").unwrap(),
            ActionOption::new("Reject", "route-reject").unwrap(),
        ])
        .unwrap();
        assert_eq!(actions.options.len(), 2);
        assert!(ActionSet::new(Vec::new()).is_err());
        assert!(ActionOption::new("Approve", " ").is_err());
    }

    #[test]
    fn inbound_text_preserves_whitespace_allowed_by_the_schema() {
        let inbound = NormalizedInbound {
            schema_version: NORMALIZED_INBOUND_SCHEMA_VERSION,
            event_id: "event".to_string(),
            address: MessagingAddress::new(
                "telegram",
                "bot",
                "chat",
                None,
                ConversationAudience::Personal,
            )
            .unwrap(),
            actor: ExternalActor::new("telegram", "bot", "user", None).unwrap(),
            content: NormalizedInboundContent::Message {
                provider_message_id: None,
                attention: MessageAttention::Unknown,
                parts: vec![InboundMessagePart::Text {
                    text: " message with spacing ".to_string(),
                }],
            },
            conversation_display_name: None,
            occurred_at_ms: None,
        };
        assert!(inbound.validate().is_ok());
    }

    #[test]
    fn inbound_rejects_more_parts_than_the_schema_allows() {
        let inbound = NormalizedInbound {
            schema_version: NORMALIZED_INBOUND_SCHEMA_VERSION,
            event_id: "event".to_string(),
            address: MessagingAddress::new(
                "telegram",
                "bot",
                "chat",
                None,
                ConversationAudience::Personal,
            )
            .unwrap(),
            actor: ExternalActor::new("telegram", "bot", "user", None).unwrap(),
            content: NormalizedInboundContent::Message {
                provider_message_id: None,
                attention: MessageAttention::Unknown,
                parts: (0..17)
                    .map(|index| InboundMessagePart::Text {
                        text: format!("part-{index}"),
                    })
                    .collect(),
            },
            conversation_display_name: None,
            occurred_at_ms: None,
        };
        assert!(inbound.validate().is_err());
    }
}
