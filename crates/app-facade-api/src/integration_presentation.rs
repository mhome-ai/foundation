//! Read-only display metadata for App UI consumers.
use crate::integration::CapabilityDescriptor;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const GET_TARGET: &str = "/app/integration/presentation/get";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetIntegrationPresentationInput {
    pub integration_id: String,
}
impl GetIntegrationPresentationInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.integration_id.trim().is_empty() {
            return Err("integrationId is required");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IntegrationPresentation {
    pub integration_id: String,
    pub integration_names: BTreeMap<String, String>,
    pub integration_icon_urls: BTreeMap<String, String>,
    pub type_names: BTreeMap<String, String>,
    pub twin_names: BTreeMap<String, String>,
    pub entity_standalone: BTreeMap<String, bool>,
    pub group_supported: BTreeMap<String, bool>,
    pub capabilities: Vec<CapabilityDescriptor>,
}
