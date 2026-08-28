use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const SURFACE_VERSION: &str = "cs1";

/// Canonical identity and delivery route of one isolated Conversation surface.
///
/// The wire representation is always the canonical string returned by
/// [`ConversationSurface::canonical_id`]. Trusted ingress owns construction;
/// clients and provider payloads must not choose a surface directly.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConversationSurface {
    ClientPersonal {
        user_id: String,
    },
    ClientGroup {
        group_id: String,
    },
    MessagingPersonal {
        provider: String,
        account_id: String,
        conversation_id: String,
        lane_id: Option<String>,
    },
    MessagingGroup {
        provider: String,
        account_id: String,
        conversation_id: String,
        lane_id: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceParseError;

impl fmt::Display for SurfaceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid canonical Conversation surface")
    }
}

impl std::error::Error for SurfaceParseError {}

impl ConversationSurface {
    pub fn client_personal(user_id: impl Into<String>) -> Result<Self, SurfaceParseError> {
        Ok(Self::ClientPersonal {
            user_id: required(user_id.into())?,
        })
    }

    pub fn client_group(group_id: impl Into<String>) -> Result<Self, SurfaceParseError> {
        Ok(Self::ClientGroup {
            group_id: required(group_id.into())?,
        })
    }

    pub fn messaging_personal(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        conversation_id: impl Into<String>,
        lane_id: Option<String>,
    ) -> Result<Self, SurfaceParseError> {
        Ok(Self::MessagingPersonal {
            provider: normalize_provider(provider.into())?,
            account_id: required(account_id.into())?,
            conversation_id: required(conversation_id.into())?,
            lane_id: optional(lane_id)?,
        })
    }

    pub fn messaging_group(
        provider: impl Into<String>,
        account_id: impl Into<String>,
        conversation_id: impl Into<String>,
        lane_id: Option<String>,
    ) -> Result<Self, SurfaceParseError> {
        Ok(Self::MessagingGroup {
            provider: normalize_provider(provider.into())?,
            account_id: required(account_id.into())?,
            conversation_id: required(conversation_id.into())?,
            lane_id: optional(lane_id)?,
        })
    }

    #[must_use]
    pub fn canonical_id(&self) -> String {
        match self {
            Self::ClientPersonal { user_id } => {
                format!("{SURFACE_VERSION}:cp:{}", encode(user_id))
            }
            Self::ClientGroup { group_id } => {
                format!("{SURFACE_VERSION}:cg:{}", encode(group_id))
            }
            Self::MessagingPersonal {
                provider,
                account_id,
                conversation_id,
                lane_id,
            } => messaging_id(
                "mp",
                provider,
                account_id,
                conversation_id,
                lane_id.as_deref(),
            ),
            Self::MessagingGroup {
                provider,
                account_id,
                conversation_id,
                lane_id,
            } => messaging_id(
                "mg",
                provider,
                account_id,
                conversation_id,
                lane_id.as_deref(),
            ),
        }
    }

    #[must_use]
    pub fn is_personal(&self) -> bool {
        matches!(
            self,
            Self::ClientPersonal { .. } | Self::MessagingPersonal { .. }
        )
    }

    #[must_use]
    pub fn is_group(&self) -> bool {
        !self.is_personal()
    }

    #[must_use]
    pub fn is_client(&self) -> bool {
        matches!(self, Self::ClientPersonal { .. } | Self::ClientGroup { .. })
    }

    #[must_use]
    pub fn is_messaging(&self) -> bool {
        !self.is_client()
    }

    #[must_use]
    pub fn user_id(&self) -> Option<&str> {
        match self {
            Self::ClientPersonal { user_id } => Some(user_id),
            _ => None,
        }
    }

    #[must_use]
    pub fn group_id(&self) -> Option<&str> {
        match self {
            Self::ClientGroup { group_id } => Some(group_id),
            _ => None,
        }
    }

    #[must_use]
    pub fn messaging_route(&self) -> Option<MessagingSurfaceRoute<'_>> {
        match self {
            Self::MessagingPersonal {
                provider,
                account_id,
                conversation_id,
                lane_id,
            }
            | Self::MessagingGroup {
                provider,
                account_id,
                conversation_id,
                lane_id,
            } => Some(MessagingSurfaceRoute {
                provider,
                account_id,
                conversation_id,
                lane_id: lane_id.as_deref(),
                group: self.is_group(),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessagingSurfaceRoute<'a> {
    pub provider: &'a str,
    pub account_id: &'a str,
    pub conversation_id: &'a str,
    pub lane_id: Option<&'a str>,
    pub group: bool,
}

impl fmt::Display for ConversationSurface {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.canonical_id())
    }
}

impl FromStr for ConversationSurface {
    type Err = SurfaceParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let parts = value.split(':').collect::<Vec<_>>();
        if parts.first().copied() != Some(SURFACE_VERSION) {
            return Err(SurfaceParseError);
        }
        let surface = match parts.as_slice() {
            [_, "cp", user_id] => Self::client_personal(decode(user_id)?),
            [_, "cg", group_id] => Self::client_group(decode(group_id)?),
            [_, kind @ ("mp" | "mg"), provider, account_id, conversation_id] => messaging(
                kind,
                provider,
                decode(account_id)?,
                decode(conversation_id)?,
                None,
            ),
            [_, kind @ ("mp" | "mg"), provider, account_id, conversation_id, lane_id] => messaging(
                kind,
                provider,
                decode(account_id)?,
                decode(conversation_id)?,
                Some(decode(lane_id)?),
            ),
            _ => Err(SurfaceParseError),
        }?;
        (surface.canonical_id() == value)
            .then_some(surface)
            .ok_or(SurfaceParseError)
    }
}

impl Serialize for ConversationSurface {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.canonical_id())
    }
}

impl<'de> Deserialize<'de> for ConversationSurface {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

fn messaging(
    kind: &str,
    provider: &str,
    account_id: String,
    conversation_id: String,
    lane_id: Option<String>,
) -> Result<ConversationSurface, SurfaceParseError> {
    match kind {
        "mp" => {
            ConversationSurface::messaging_personal(provider, account_id, conversation_id, lane_id)
        }
        "mg" => {
            ConversationSurface::messaging_group(provider, account_id, conversation_id, lane_id)
        }
        _ => Err(SurfaceParseError),
    }
}

fn messaging_id(
    kind: &str,
    provider: &str,
    account_id: &str,
    conversation_id: &str,
    lane_id: Option<&str>,
) -> String {
    let mut value = format!(
        "{SURFACE_VERSION}:{kind}:{provider}:{}:{}",
        encode(account_id),
        encode(conversation_id)
    );
    if let Some(lane_id) = lane_id {
        value.push(':');
        value.push_str(&encode(lane_id));
    }
    value
}

fn required(value: String) -> Result<String, SurfaceParseError> {
    let trimmed = value.trim();
    (!trimmed.is_empty() && trimmed == value)
        .then_some(value)
        .ok_or(SurfaceParseError)
}

fn optional(value: Option<String>) -> Result<Option<String>, SurfaceParseError> {
    value.map(required).transpose()
}

fn normalize_provider(value: String) -> Result<String, SurfaceParseError> {
    let normalized = value.trim().to_ascii_lowercase();
    (!normalized.is_empty()
        && normalized
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
        && normalized
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'))
    .then_some(normalized)
    .ok_or(SurfaceParseError)
}

fn encode(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value.as_bytes())
}

fn decode(value: &str) -> Result<String, SurfaceParseError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| SurfaceParseError)?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return Err(SurfaceParseError);
    }
    required(String::from_utf8(decoded).map_err(|_| SurfaceParseError)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_surface_variants_round_trip_canonically() {
        let surfaces = [
            ConversationSurface::client_personal("user:1").unwrap(),
            ConversationSurface::client_group("group:1").unwrap(),
            ConversationSurface::messaging_personal("Telegram", "bot:1", "chat:2", None).unwrap(),
            ConversationSurface::messaging_group(
                "feishu",
                "bot:1",
                "chat:2",
                Some("topic:3".to_string()),
            )
            .unwrap(),
        ];
        for surface in surfaces {
            let encoded = surface.canonical_id();
            assert_eq!(encoded.parse::<ConversationSurface>().unwrap(), surface);
            assert_eq!(
                serde_json::from_str::<ConversationSurface>(
                    &serde_json::to_string(&surface).unwrap()
                )
                .unwrap(),
                surface
            );
        }
    }

    #[test]
    fn surface_kind_and_routes_are_typed() {
        let personal = ConversationSurface::client_personal("user").unwrap();
        assert!(personal.is_personal());
        assert!(personal.is_client());
        assert_eq!(personal.user_id(), Some("user"));

        let group = ConversationSurface::messaging_group(
            "telegram",
            "account",
            "chat",
            Some("topic".to_string()),
        )
        .unwrap();
        assert!(group.is_group());
        let route = group.messaging_route().unwrap();
        assert_eq!(route.provider, "telegram");
        assert_eq!(route.lane_id, Some("topic"));
        assert!(route.group);
    }

    #[test]
    fn noncanonical_or_ambiguous_surfaces_are_rejected() {
        for value in [
            "meow-link",
            "cs1:cp:",
            "cs1:cp:dXNlcg==",
            "cs1:mp:Telegram:YQ:Yg",
            "cs1:mg:telegram:YQ:Yg:",
            "cs2:cp:dXNlcg",
        ] {
            assert!(value.parse::<ConversationSurface>().is_err(), "{value}");
        }
    }
}
