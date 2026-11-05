//! Selected Automation labels for App UI; no state, script parsing or history expansion.
use crate::resource_read::ResourceIdsInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NAMES_GET_TARGET: &str = "/app/presentation/automation/names/get";
pub type NamesInput = ResourceIdsInput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamesOutput {
    pub automations: BTreeMap<String, String>,
}
