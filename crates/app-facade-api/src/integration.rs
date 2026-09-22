//! Capability presentation metadata shared by all Integration views.
//!
//! IDs are fully qualified. `parentCapabilityId` is the direct schema override,
//! never a self-reference. The server resolves presentation fields, including
//! `tag`, before returning a descriptor. Missing tag means no primary tag.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_capability_id: Option<String>,
    #[serde(rename = "type")]
    pub kind: String,
    pub display_name: String,
    pub description: String,
    pub visibility: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

impl CapabilityDescriptor {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_capability_id(&self.capability_id) {
            return Err("capabilityId must be a fully qualified capability ID");
        }
        if let Some(parent) = &self.parent_capability_id {
            if !valid_capability_id(parent) || parent == &self.capability_id {
                return Err("parentCapabilityId must identify another capability");
            }
            if parent
                .split('.')
                .skip(2)
                .ne(self.capability_id.split('.').skip(2))
            {
                return Err("parentCapabilityId must have the same method and capability name");
            }
        }
        let method = self.capability_id.split('.').nth(2).unwrap_or_default();
        if self.kind != method.to_ascii_uppercase() {
            return Err("type must match the capability method");
        }
        if !matches!(self.visibility.as_str(), "full" | "internal") {
            return Err("invalid capability visibility");
        }
        if self
            .tag
            .as_ref()
            .is_some_and(|tag| tag.is_empty() || tag.trim() != tag)
        {
            return Err("tag must be a non-empty tag reference");
        }
        Ok(())
    }
}

fn valid_capability_id(id: &str) -> bool {
    let parts: Vec<_> = id.split('.').collect();
    parts.len() == 4
        && parts
            .iter()
            .all(|part| !part.is_empty() && !part.chars().any(char::is_whitespace))
        && matches!(parts[2], "command" | "state" | "event" | "data")
}
