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

pub const SUMMARIES_GET: &str = "/app/presentation/automation/summaries/get";
pub const EDITOR_INIT: &str = "/app/presentation/automation/overview/init";
pub const EDITOR_CONFIG_UPDATE: &str = "/app/presentation/automation/overview/config/update";
pub const EDITOR_CONFIG_PROVIDE: &str = "/app/presentation/automation/overview/config/provide";
pub const EDITOR_SAVE: &str = "/app/presentation/automation/overview/save";
pub const EDITOR_CANCEL: &str = "/app/presentation/automation/overview/cancel";
pub const COPY: &str = "/app/presentation/automation/copy";
pub const ENROLL: &str = "/app/presentation/automation/enroll";
pub const SHARE: &str = "/app/presentation/automation/share";
pub const GENERATED_ENABLE: &str = "/app/presentation/automation/generated/enable";
pub const GENERATED_DISABLE: &str = "/app/presentation/automation/generated/disable";
pub const GENERATED_RUN: &str = "/app/presentation/automation/generated/run";

/// Selected UI state and labels. No authored source, parsing, Widget or snapshot reads.
pub type SummariesInput = ResourceIdsInput;
pub type SummariesOutput = crate::resource_read::ResourceReadOutput<Summary>;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    #[serde(flatten)]
    pub item: ListItem,
    pub automation_id: String,
    pub temp: bool,
    pub last_run_at: Option<i64>,
    pub expire_at: Option<i64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CopyInput {
    pub automation_id: String,
    pub name: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedOutput {
    pub automation_id: String,
    pub runnable: bool,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedRunInput {
    pub automation_id: String,
    pub timezone: Option<String>,
    pub location_symbol: Option<String>,
}
/// Generated execution is an explicit internal UI operation, never a public start alias.
pub fn generated_program_id(id: &str) -> Result<&str, &'static str> {
    let id = id.trim();
    if id.len() > 2
        && ["P-", "S-", "G-"]
            .iter()
            .any(|prefix| id.starts_with(prefix))
    {
        Ok(id)
    } else {
        Err("A generated Automation program ID is required")
    }
}
