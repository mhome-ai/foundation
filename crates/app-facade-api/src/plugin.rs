use serde::{Deserialize, Serialize};

pub const TARGET_PREFIX: &str = "/app/plugin";
pub const CATALOG_LIST_TARGET: &str = "/app/plugin/catalog/list";
pub const EXTENSION_VERSION: &str = "v1";
pub const ENABLED_LIST_TARGET: &str = "/app/plugin/enabled/list";
pub const DETAIL_GET_TARGET: &str = "/app/plugin/detail/get";
pub const CANDIDATE_LIST_TARGET: &str = "/app/plugin/candidate/list";
pub const ENABLE_START_TARGET: &str = "/app/plugin/enable/start";
pub const ENABLE_STATUS_TARGET: &str = "/app/plugin/enable/status";
pub const DISABLE_TARGET: &str = "/app/plugin/disable";
pub const PROFILE_UPDATE_TARGET: &str = "/app/plugin/profile/update";
/// Read-only Local Space management check before the CLI changes its local Host.
pub const PREVIEW_CHECK_TARGET: &str = "/app/plugin/preview/check";
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

pub const INSTALL_PREPARE_TARGET: &str = "/app/plugin/install/prepare";
pub const INSTALL_START_TARGET: &str = "/app/plugin/install/start";
pub const INSTALL_STATUS_TARGET: &str = "/app/plugin/install/status";

/// Space Manage entrypoint. Native Preview carries the local Host receipt;
/// ordinary published onboarding omits it and keeps publisher verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartPluginEnableInput {
    pub node_type: String,
    pub candidate_id: String,
    pub host_id: Option<String>,
    pub preview: Option<core_api::plugin_preview::PluginPreviewExpectation>,
}

/// This check does not create an installation or reserve permission for later calls.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckPluginPreviewInput {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CheckPluginPreviewOutput {
    pub space_id: String,
}
