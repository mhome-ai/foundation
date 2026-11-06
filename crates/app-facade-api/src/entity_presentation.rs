//! UI presentation reads. Business Entity/Device detail contracts remain separate.
use serde::{Deserialize, Serialize};

pub const GET_TARGET: &str = "/app/presentation/entity/get";
pub const META_GET_TARGET: &str = "/app/presentation/entity/meta/get";

/// Selected reference identities, without state, members, cards or execution data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetEntityMetadataInput {
    pub entity_type: String,
    pub ids: Vec<String>,
}

impl GetEntityMetadataInput {
    pub fn normalized(&self) -> Result<(String, Vec<String>), &'static str> {
        let entity_type = self.entity_type.trim();
        if entity_type.is_empty() {
            return Err("entityType is required");
        }
        let ids = crate::resource_read::ResourceIdsInput {
            ids: self.ids.clone(),
        }
        .normalized_ids()?;
        Ok((entity_type.to_string(), ids))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityMetadata {
    pub id: String,
    pub entity_type: String,
    pub display_name: String,
    pub group: bool,
}

pub type EntityMetadataOutput = crate::resource_read::ResourceReadOutput<EntityMetadata>;

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
