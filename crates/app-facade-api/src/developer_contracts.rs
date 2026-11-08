//! Cloud-standard contracts acquired through an authorized, online Hub.
//!
//! A Space selects the transport and grants access to the Hub. It never selects
//! the schema channel, installed version or Preview used by these operations.
use serde::{Deserialize, Serialize};

pub const LIST_TARGET: &str = "/app/developer/contracts/list";
pub const EXPORT_TARGET: &str = "/app/developer/contracts/export";
pub const BRIDGE_LIST_TARGET: &str = "/bridge/schema/contracts/list";
pub const BRIDGE_EXPORT_TARGET: &str = "/bridge/schema/contracts/export";
pub const MAX_SELECTED: usize = 64;
// Leave room for the Facade envelope and JSON escaping on the 2 MiB Client socket.
pub const MAX_RESULT_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListRequest {
    pub integration_id: String,
}

impl ListRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_integration_id(&self.integration_id) {
            return Err("integrationId must identify a standard Integration");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    pub selected: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_none_match: Option<String>,
}

impl ExportRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.selected.is_empty() || self.selected.len() > MAX_SELECTED {
            return Err("selected must contain 1..64 unique capability IDs");
        }
        let mut unique = std::collections::BTreeSet::new();
        for id in &self.selected {
            let parts: Vec<_> = id.split('.').collect();
            if parts.len() != 4
                || !parts.iter().all(|part| identifier(part))
                || !matches!(parts[2], "command" | "state" | "event" | "data")
                || !unique.insert(id)
            {
                return Err("selected must contain unique fully qualified capability IDs");
            }
        }
        if self
            .if_none_match
            .as_ref()
            .is_some_and(|value| !sha256(value))
        {
            return Err("ifNoneMatch must be a lowercase SHA-256 digest");
        }
        Ok(())
    }
}

pub fn valid_integration_id(value: &str) -> bool {
    let parts: Vec<_> = value.split('.').collect();
    parts.len() == 2 && parts.iter().all(|part| identifier(part))
}

fn identifier(value: &str) -> bool {
    value.len() <= 128
        && value
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphabetic)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
}

fn sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// The compiler-owned document retains its `meow-capability-contracts` format.
/// This transport wrapper does not reinterpret or project away its fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ExportResponse {
    Ready { document: serde_json::Value },
    Unchanged { sha256: String },
}
