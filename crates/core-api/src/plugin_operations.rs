//! Core-owned context for a declared public operation. Never part of business arguments.
use serde::{Deserialize, Serialize};

pub const PUBLIC_OPERATIONS_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicOperationPhase {
    Prepare,
    Invoke,
    Submit,
    Status,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicOperationControl {
    pub contract_id: String,
    pub operation_id: String,
    pub invocation_id: String,
    pub owner: String,
    pub source_manifest_sha256: String,
    pub phase: PublicOperationPhase,
    pub approved: bool,
}

impl PublicOperationControl {
    pub fn validate(&self) -> Result<(), &'static str> {
        if [&self.contract_id, &self.operation_id, &self.invocation_id]
            .iter()
            .any(|value| {
                value.is_empty() || value.len() > 256 || value.chars().any(char::is_control)
            })
            || !crate::plugin_preview::is_sha256(&self.owner)
            || !crate::plugin_preview::is_sha256(&self.source_manifest_sha256)
        {
            return Err("Invalid public Plugin operation context");
        }
        Ok(())
    }
}
