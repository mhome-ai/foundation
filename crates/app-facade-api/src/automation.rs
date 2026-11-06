//! Shared user-program control and lightweight display-name contracts.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const START: &str = "/app/automation/start";
pub const STOP: &str = "/app/automation/stop";
pub const RENAME: &str = "/app/automation/rename";
pub const DESCRIPTION_UPDATE: &str = "/app/automation/description/update";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartInput {
    pub automation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_symbol: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopInput {
    pub automation_id: String,
}

/// Observed control state, not an assertion that a Task Flow is still executing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlOutput {
    pub automation_id: String,
    pub enabled: bool,
    pub running: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameInput {
    pub automation_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameOutput {
    pub automation_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DescriptionInput {
    pub automation_id: String,
    pub expected_script_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub var_description: BTreeMap<String, String>,
}
