//! UI presentation reads. Business Entity/Device detail contracts remain separate.
use serde::{Deserialize, Serialize};

pub const GET_TARGET: &str = "/app/entity/presentation/get";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetEntityPresentationInput {
    pub entity_type: String,
    pub entity_id: String,
}

impl GetEntityPresentationInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.entity_type.trim().is_empty() || self.entity_id.trim().is_empty() {
            return Err("entityType and entityId are required");
        }
        Ok(())
    }
}
