use serde::{Deserialize, Serialize};

pub const TARGET_PREFIX: &str = "/app/plugin";
pub const EXTENSION_VERSION: &str = "v1";
pub const INSTALLED_LIST_TARGET: &str = "/app/plugin/installed/list";
pub const DETAIL_GET_TARGET: &str = "/app/plugin/detail/get";
pub const CANDIDATE_LIST_TARGET: &str = "/app/plugin/candidate/list";
pub const ADD_START_TARGET: &str = "/app/plugin/add/start";
pub const ADD_STATUS_TARGET: &str = "/app/plugin/add/status";
pub const ADD_CANCEL_TARGET: &str = "/app/plugin/add/cancel";
pub const REMOVE_TARGET: &str = "/app/plugin/remove";
pub const PROFILE_UPDATE_TARGET: &str = "/app/plugin/profile/update";
pub const SETTINGS_CHANGED_TARGET: &str = "/app/plugin/settings/changed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginNodeRequest<T> {
    pub node_id: String,
    pub version: String,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginNodeResponse<T> {
    pub node_id: String,
    pub node_type: String,
    pub version: String,
    pub data: T,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsChangedEvent {
    pub version: String,
    pub node_id: String,
    pub node_type: String,
    pub event_seq: u64,
    pub section: String,
    pub revision: u64,
}
