use serde::{Deserialize, Serialize};

pub const STATUS_CONTRACT: &str = "mhome.runtime.status.v1";
pub const STATUS_LIST_TARGET: &str = "/app/runtime/status/list";
pub const STATUS_CHANGED_TARGET: &str = "/app/runtime/status/changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatusResourceType {
    NodeInstance,
    NodeRuntime,
    ManagedService,
    HubConnection,
    ScopeConnection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusResource {
    pub r#type: StatusResourceType,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub contract: String,
    pub scope_id: String,
    pub hub_generation: String,
    pub hub_revision: u64,
    pub observed_at_ms: i64,
    pub resource: StatusResource,
    pub deleted: bool,
    #[serde(default)]
    pub snapshot: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusListResponse {
    pub contract: String,
    pub scope_id: String,
    pub hub_generation: String,
    #[serde(default)]
    pub resources: Vec<StatusPayload>,
    pub observed_at_ms: i64,
}
