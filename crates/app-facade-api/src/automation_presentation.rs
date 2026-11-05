//! Selected Automation labels for App UI; no state, script parsing or history expansion.
use crate::resource_read::ResourceIdsInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const NAMES_GET_TARGET: &str = "/app/presentation/automation/names/get";
pub const LIST_TARGET: &str = "/app/presentation/automation/list";
pub const OVERVIEW_GET_TARGET: &str = "/app/presentation/automation/overview/get";
pub const PLACEMENTS_GET_TARGET: &str = "/app/presentation/automation/placements/get";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProgramType {
    Recurring,
    TaskFlow,
}
impl ProgramType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Recurring => "RECURRING",
            Self::TaskFlow => "TASK_FLOW",
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListInput {
    pub r#type: ProgramType,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationInput {
    pub automation_id: String,
}
impl AutomationInput {
    pub fn normalized_id(&self) -> Result<&str, &'static str> {
        let id = self.automation_id.trim();
        if id.is_empty() {
            Err("automationId is required")
        } else {
            Ok(id)
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOutput {
    pub automations: Vec<ListItem>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub enabled: bool,
    pub corrupted: bool,
    pub runnable: bool,
    pub running: bool,
    pub blocked: bool,
    pub has_expire: bool,
    pub timestamp: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub enabled_at: i64,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OverviewOutput {
    pub id: String,
    pub name: String,
    pub r#type: String,
    pub enabled: bool,
    pub runnable: bool,
    pub corrupted: bool,
    pub blocked: bool,
    pub widgets: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CanvasPlacement {
    pub id: String,
    pub name: String,
    pub contains_automation: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlacementsOutput {
    pub canvases: Vec<CanvasPlacement>,
}
pub type NamesInput = ResourceIdsInput;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NamesOutput {
    pub automations: BTreeMap<String, String>,
}
