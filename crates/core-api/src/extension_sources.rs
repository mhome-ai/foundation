//! Space-owned Integration source selection. A source identity is not authority.
//! Preview bytes remain on the bound Hub; published bytes remain in the registry.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SOURCE_SNAPSHOT_ROUTE: &str = "/dispatch/extension/sources/snapshot";
pub const SOURCE_READ_ROUTE: &str = "/dispatch/extension/sources/read";
pub const MAX_SOURCE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_SELECTION_BYTES: usize = 256 * 1024;

/// Exact SHA-256 identity with the same lowercase hexadecimal wire form on both runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotId([u8; 32]);

impl std::str::FromStr for SnapshotId {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64
            || !value
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err("invalid preview source snapshot");
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| "invalid preview source snapshot")?;
        }
        Ok(Self(bytes))
    }
}
impl std::fmt::Display for SnapshotId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}
impl Serialize for SnapshotId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}
impl<'de> Deserialize<'de> for SnapshotId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationSource {
    pub integration_id: String,
    pub version: u32,
    #[serde(rename = "type")]
    pub package_type: PackageType,
    /// Exact local source manifest digest. Absent for registry-owned releases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<SnapshotId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PackageType {
    Standard,
    Composite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpaceSources {
    pub revision: u64,
    pub integrations: Vec<IntegrationSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReadRequest {
    pub source: IntegrationSource,
}

/// Only source text and exact script digests cross the compiler callback. Script
/// bodies are resolved separately on the Hub and never enter the parsed graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceReadResponse {
    pub source: IntegrationSource,
    pub schemas: BTreeMap<String, String>,
    pub script_digests: BTreeMap<String, String>,
}

impl IntegrationSource {
    pub fn validate(&self) -> Result<(), &'static str> {
        let parts: Vec<_> = self.integration_id.split('.').collect();
        if self.integration_id != "base"
            && (parts.len() != 2
                || parts.iter().any(|part| {
                    part.is_empty()
                        || !part.as_bytes()[0].is_ascii_alphabetic()
                        || !part
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                }))
        {
            return Err("invalid Integration source identity");
        }
        if self.integration_id.len() > 240 || self.version == 0 || self.version > i32::MAX as u32 {
            return Err("invalid Integration source version");
        }
        if self.snapshot.is_some() {
            if self.integration_id == "base" || self.package_type != PackageType::Standard {
                return Err("invalid preview source snapshot");
            }
        }
        Ok(())
    }
}

impl SpaceSources {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.revision > 9_007_199_254_740_991
            || self.integrations.len() > 1000
            || (self.revision == 0 && !self.integrations.is_empty())
        {
            return Err("invalid Space source snapshot");
        }
        let mut sources = std::collections::HashSet::new();
        for source in &self.integrations {
            source.validate()?;
            if !sources.insert((&source.integration_id, source.package_type)) {
                return Err("conflicting Integration sources");
            }
        }
        if serde_json::to_vec(self)
            .map_err(|_| "invalid Space source snapshot")?
            .len()
            > MAX_SELECTION_BYTES
        {
            return Err("Space source selection exceeds the size limit");
        }
        Ok(())
    }
}

impl SourceReadResponse {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.source.validate()?;
        if self.schemas.is_empty() || self.schemas.len() > 1000 || self.script_digests.len() > 1000
        {
            return Err("invalid Integration source inventory");
        }
        for (name, content) in &self.schemas {
            if name.is_empty()
                || name.len() > 1024
                || name.contains(['\\', ':', '\0'])
                || name
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || !(name.ends_with(".yml") || name.ends_with(".yaml"))
                || content.is_empty()
            {
                return Err("invalid Integration source file");
            }
        }
        for (function, digest) in &self.script_digests {
            if function.is_empty()
                || function.len() > 128
                || !function.bytes().enumerate().all(|(i, c)| {
                    c.is_ascii_alphabetic() || c == b'_' || (i > 0 && c.is_ascii_digit())
                })
                || digest.len() != 64
                || !digest
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err("invalid Integration script digest");
            }
        }
        // Bound encoded bytes, including JSON escaping, below the general Bridge
        // payload ceiling. Large responses use its existing object-store transport.
        if serde_json::to_vec(self)
            .map_err(|_| "invalid Integration source")?
            .len()
            > MAX_SOURCE_BYTES
        {
            return Err("Integration source exceeds the size limit");
        }
        Ok(())
    }
}
