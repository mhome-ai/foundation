//! Stable recipient identities, independent of placement and live connections.
//! Conversation surfaces identify Agent conversations; these IDs address recipients.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use conversation_api::ConversationSurface;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RecipientId(String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipientIdError;

impl fmt::Display for RecipientIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid canonical recipient identity")
    }
}
impl std::error::Error for RecipientIdError {}

impl RecipientId {
    /// Convert a validated Messaging surface without losing audience or lane.
    pub fn messaging(surface: &ConversationSurface) -> Result<Self, RecipientIdError> {
        let surface_id = surface.canonical_id();
        let id = if let Some(address) = surface_id.strip_prefix("cs1:mp:") {
            format!("m:p:{address}")
        } else if let Some(address) = surface_id.strip_prefix("cs1:mg:") {
            format!("m:g:{address}")
        } else {
            return Err(RecipientIdError);
        };
        id.parse()
    }

    pub fn node(node_type: &str, node_id: &str) -> Result<Self, RecipientIdError> {
        if !valid_kind(node_type) || !valid_segment(node_id) {
            return Err(RecipientIdError);
        }
        Ok(Self(format!(
            "n:{node_type}:{}",
            URL_SAFE_NO_PAD.encode(node_id)
        )))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn messaging_surface(&self) -> Option<ConversationSurface> {
        surface_id(&self.0)?.parse().ok()
    }

    /// The base authorization route excludes a provider topic/lane.
    /// This does not change the recipient used for the actual delivery.
    pub fn messaging_base(&self) -> Option<Self> {
        let surface = match self.messaging_surface()? {
            ConversationSurface::MessagingPersonal {
                provider,
                account_id,
                conversation_id,
                ..
            } => {
                ConversationSurface::messaging_personal(provider, account_id, conversation_id, None)
                    .ok()?
            }
            ConversationSurface::MessagingGroup {
                provider,
                account_id,
                conversation_id,
                ..
            } => ConversationSurface::messaging_group(provider, account_id, conversation_id, None)
                .ok()?,
            _ => return None,
        };
        Self::messaging(&surface).ok()
    }

    pub fn node_address(&self) -> Option<(&str, String)> {
        let tail = self.0.strip_prefix("n:")?;
        let (kind, encoded) = tail.split_once(':')?;
        Some((kind, decode(encoded).ok()?))
    }
}

fn surface_id(id: &str) -> Option<String> {
    if let Some(address) = id.strip_prefix("m:p:") {
        Some(format!("cs1:mp:{address}"))
    } else {
        id.strip_prefix("m:g:")
            .map(|address| format!("cs1:mg:{address}"))
    }
}

fn valid_kind(value: &str) -> bool {
    let mut bytes = value.bytes();
    bytes.next().is_some_and(|first| first.is_ascii_lowercase())
        && bytes.all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
fn valid_segment(value: &str) -> bool {
    !value.is_empty() && value.trim() == value
}
fn decode(encoded: &str) -> Result<String, RecipientIdError> {
    let value = String::from_utf8(
        URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| RecipientIdError)?,
    )
    .map_err(|_| RecipientIdError)?;
    if !valid_segment(&value) || URL_SAFE_NO_PAD.encode(&value) != encoded {
        return Err(RecipientIdError);
    }
    Ok(value)
}

impl FromStr for RecipientId {
    type Err = RecipientIdError;
    fn from_str(id: &str) -> Result<Self, Self::Err> {
        if let Some(surface_id) = surface_id(id) {
            let surface: ConversationSurface = surface_id.parse().map_err(|_| RecipientIdError)?;
            if surface.canonical_id() != surface_id {
                return Err(RecipientIdError);
            }
        } else if let Some(tail) = id.strip_prefix("n:") {
            let (kind, encoded) = tail.split_once(':').ok_or(RecipientIdError)?;
            if !valid_kind(kind) {
                return Err(RecipientIdError);
            }
            decode(encoded)?;
        } else {
            return Err(RecipientIdError);
        }
        Ok(Self(id.to_owned()))
    }
}
impl fmt::Display for RecipientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl Serialize for RecipientId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}
impl<'de> Deserialize<'de> for RecipientId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_conformance_vectors() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/recipient-id.conformance.json"))
                .unwrap();
        for case in fixtures["valid"].as_array().unwrap() {
            let id: RecipientId = case["id"].as_str().unwrap().parse().unwrap();
            assert_eq!(serde_json::to_value(&id).unwrap(), case["id"]);
            if let Some(surface) = case.get("surface") {
                let parsed: ConversationSurface = surface.as_str().unwrap().parse().unwrap();
                assert_eq!(RecipientId::messaging(&parsed).unwrap(), id);
                assert_eq!(id.messaging_surface().unwrap(), parsed);
            } else {
                let (kind, node) = id.node_address().unwrap();
                assert_eq!(kind, case["nodeType"].as_str().unwrap());
                assert_eq!(node, case["nodeId"].as_str().unwrap());
                assert_eq!(RecipientId::node(kind, &node).unwrap(), id);
            }
        }
        for case in fixtures["invalid"].as_array().unwrap() {
            assert!(
                case.as_str().unwrap().parse::<RecipientId>().is_err(),
                "{case}"
            );
        }
    }
    #[test]
    fn lane_delivery_and_base_authorization_are_distinct() {
        let id: RecipientId = "m:g:telegram:Ym90:Y2hhdA:dG9waWM".parse().unwrap();
        assert_eq!(
            id.messaging_base().unwrap().as_str(),
            "m:g:telegram:Ym90:Y2hhdA"
        );
        assert_eq!(id.as_str(), "m:g:telegram:Ym90:Y2hhdA:dG9waWM");
    }
}
