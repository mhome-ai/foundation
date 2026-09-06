use serde::{Deserialize, Serialize};

pub const STATUS_CONTRACT: &str = "mhome.runtime.status.v1";
pub const STATUS_LIST_TARGET: &str = "/app/runtime/status/list";
pub const STATUS_CHANGED_TARGET: &str = "/app/runtime/status/changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatusResourceType {
    NodeInstance,
    NodeRuntime,
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

/// Optional filters for the Hub-owned runtime status projection. An empty
/// request lists every Node instance and logical Node runtime in the scope.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusListRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_type: Option<StatusResourceType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_runtime_resources_are_only_hub_normalized_node_state() {
        assert_eq!(
            serde_json::to_value(StatusResourceType::NodeInstance).unwrap(),
            "nodeInstance"
        );
        assert_eq!(
            serde_json::to_value(StatusResourceType::NodeRuntime).unwrap(),
            "nodeRuntime"
        );
        assert!(serde_json::from_str::<StatusResourceType>("\"hubConnection\"").is_err());
        assert!(serde_json::from_str::<StatusResourceType>("\"managedService\"").is_err());
    }

    #[test]
    fn empty_status_list_request_has_no_filters() {
        let request: StatusListRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(request, StatusListRequest::default());
        assert_eq!(serde_json::to_string(&request).unwrap(), "{}");
    }
}
