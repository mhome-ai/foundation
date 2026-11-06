//! NFC management. Physical tag IO and authenticated tap ingress are separate protocols.
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashSet;

pub const LIST: &str = "/app/nfc/list";
pub const SOURCES_LIST: &str = "/app/nfc/sources/list";
pub const CREATE: &str = "/app/nfc/create";
pub const ACTIVATE: &str = "/app/nfc/activate";
pub const UPDATE: &str = "/app/nfc/update";
pub const DELETE: &str = "/app/nfc/delete";
pub const TARGETS: &[&str] = &[LIST, SOURCES_LIST, CREATE, ACTIVATE, UPDATE, DELETE];

fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListState {
    #[default]
    Active,
    Pending,
    All,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    #[serde(default)]
    pub create_state: ListState,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourcesInput {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Policy {
    AllPhone,
    CustomPhone,
}

impl Policy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AllPhone => "ALL_PHONE",
            Self::CustomPhone => "CUSTOM_PHONE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrustedSource {
    pub source_type: String,
    pub source_id: String,
    #[serde(default)]
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub policy: Policy,
    #[serde(default)]
    pub trusted_sources: Vec<TrustedSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActivateInput {
    pub tag_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateInput {
    pub tag_id: String,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub name: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub description: Option<String>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub policy: Option<Policy>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub trusted_sources: Option<Vec<TrustedSource>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeleteInput {
    pub tag_ids: Vec<String>,
}

pub fn required(value: &str, field: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(value.to_owned())
    }
}

impl CreateInput {
    pub fn normalize(&mut self) -> Result<(), String> {
        self.name = required(&self.name, "name")?;
        normalize_sources(self.policy, &mut self.trusted_sources)
    }
}

impl UpdateInput {
    pub fn normalize(&mut self) -> Result<(), String> {
        self.tag_id = required(&self.tag_id, "tagId")?;
        if let Some(name) = &mut self.name {
            *name = required(name, "name")?;
        }
        if self.name.is_none()
            && self.description.is_none()
            && self.policy.is_none()
            && self.trusted_sources.is_none()
        {
            return Err("At least one NFC configuration field is required".into());
        }
        if let Some(sources) = &mut self.trusted_sources {
            validate_sources(sources)?;
        }
        // Validate the resulting policy and nonempty sources after merging with the stored tag.
        Ok(())
    }
}

impl DeleteInput {
    pub fn normalize(&mut self) -> Result<(), String> {
        if self.tag_ids.is_empty() || self.tag_ids.len() > 100 {
            return Err("tagIds must contain 1–100 IDs".into());
        }
        let mut seen = HashSet::new();
        for id in &mut self.tag_ids {
            *id = required(id, "tagId")?;
            if !seen.insert(id.clone()) {
                return Err("tagIds must be unique".into());
            }
        }
        Ok(())
    }
}

fn validate_sources(sources: &mut [TrustedSource]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for source in sources.iter_mut() {
        if source.source_type != "phone" {
            return Err("sourceType must be phone".into());
        }
        source.source_id = required(&source.source_id, "sourceId")?;
        if !seen.insert(source.source_id.clone()) {
            return Err("trustedSources must have unique source IDs".into());
        }
    }
    Ok(())
}

pub fn normalize_sources(policy: Policy, sources: &mut Vec<TrustedSource>) -> Result<(), String> {
    validate_sources(sources)?;
    if policy == Policy::AllPhone {
        sources.clear();
    } else if sources.is_empty() {
        return Err("CUSTOM_PHONE requires at least one trusted source".into());
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub tag_id: String,
    pub scope_id: String,
    pub name: String,
    pub description: String,
    pub policy: Policy,
    pub trusted_sources: Vec<TrustedSource>,
    pub create_state: String,
    pub created_at: i64,
    pub last_used_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOutput {
    pub nfc_tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcesOutput {
    pub sources: Vec<TrustedSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateOutput {
    pub tag_id: String,
    pub create_state: String,
    pub write_payload: String,
}

pub fn write_payload(scope_id: &str, tag_id: &str) -> String {
    format!("mhomeai://nfc/{scope_id}:{tag_id}")
}
