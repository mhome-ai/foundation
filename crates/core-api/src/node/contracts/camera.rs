use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::node::settings::VERSION;
pub const NODE_TYPE: &str = "camera";
pub const APP_TARGET_PREFIX: &str = "/app/plugin/camera/";
pub const RUNTIME_TARGET_PREFIX: &str = "/camera/app/";
pub const DEVICE_LIST: &str = "device/list";
pub const MANAGEMENT_SNAPSHOT: &str = "management/snapshot";
pub const DEVICE_REMOVE: &str = "device/remove";
pub const WATCH_STATUS: &str = "watch/status";
pub const WATCH_SET: &str = "watch/set";
pub const APP_DEVICE_LIST_TARGET: &str = "/app/plugin/camera/device/list";
pub const APP_MANAGEMENT_SNAPSHOT_TARGET: &str = "/app/plugin/camera/management/snapshot";
pub const APP_DEVICE_REMOVE_TARGET: &str = "/app/plugin/camera/device/remove";
pub const APP_WATCH_STATUS_TARGET: &str = "/app/plugin/camera/watch/status";
pub const APP_WATCH_SET_TARGET: &str = "/app/plugin/camera/watch/set";
pub const RUNTIME_DEVICE_LIST_TARGET: &str = "/camera/app/device/list";
pub const RUNTIME_MANAGEMENT_SNAPSHOT_TARGET: &str = "/camera/app/management/snapshot";
pub const RUNTIME_DEVICE_REMOVE_TARGET: &str = "/camera/app/device/remove";
pub const RUNTIME_WATCH_STATUS_TARGET: &str = "/camera/app/watch/status";
pub const RUNTIME_WATCH_SET_TARGET: &str = "/camera/app/watch/set";
pub const DEVICE_ADD_FLOW: &str = "device.add";
pub const SETTINGS_STATUS: &str = crate::node::settings::STATUS;
pub const SETTINGS_UPDATE: &str = crate::node::settings::UPDATE;
pub const SETTINGS_REVERT: &str = crate::node::settings::REVERT;
pub const SETTINGS_RETRY: &str = crate::node::settings::RETRY;
pub const RECOGNITION_SECTION: &str = "recognition";

pub type SettingsStatusRequest = crate::node::settings::StatusRequest;
pub type RecognitionUpdateRequest = crate::node::settings::UpdateRequest<RecognitionSettings>;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceIdRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchStatusRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchSetRequest {
    pub device_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CameraDeviceHealth {
    Applying,
    Ready,
    Degraded,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderCatalogItem {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub supports_multiple_cameras: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraDevice {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manufacturer_name: Option<String>,
    pub capabilities: Vec<String>,
    pub health: CameraDeviceHealth,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnlineStateObservation {
    pub id: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WatchControlResponse {
    pub ok: bool,
    pub device_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementSnapshot {
    pub providers: Vec<ProviderCatalogItem>,
    pub devices: Vec<CameraDevice>,
    pub online_states: Vec<OnlineStateObservation>,
    pub watch_states: Vec<WatchControlResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceListResponse {
    pub devices: Vec<CameraDevice>,
    pub online_states: Vec<OnlineStateObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceMutationResponse {
    pub device_ids: Vec<String>,
    pub changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecognitionSettings {
    pub enabled: bool,
}

pub type RecognitionCommandRequest = crate::node::settings::SectionCommandRequest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_and_rust_camera_contract_agree() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../../manifest/core.v1.json")).unwrap();
        let camera = &manifest["protocols"]["cameraPlugin"];
        assert_eq!(camera["version"], VERSION);
        assert_eq!(camera["nodeType"], NODE_TYPE);
        assert_eq!(camera["appTargetPrefix"], APP_TARGET_PREFIX);
        assert_eq!(camera["runtimeTargetPrefix"], RUNTIME_TARGET_PREFIX);
        assert_eq!(camera["routes"]["deviceList"], DEVICE_LIST);
        assert_eq!(camera["routes"]["managementSnapshot"], MANAGEMENT_SNAPSHOT);
        assert_eq!(camera["routes"]["deviceRemove"], DEVICE_REMOVE);
        assert_eq!(camera["routes"]["watchStatus"], WATCH_STATUS);
        assert_eq!(camera["routes"]["watchSet"], WATCH_SET);
        assert_eq!(camera["flows"]["deviceAdd"], DEVICE_ADD_FLOW);
        assert_eq!(
            APP_DEVICE_LIST_TARGET,
            format!("{APP_TARGET_PREFIX}{DEVICE_LIST}")
        );
        assert_eq!(
            APP_MANAGEMENT_SNAPSHOT_TARGET,
            format!("{APP_TARGET_PREFIX}{MANAGEMENT_SNAPSHOT}")
        );
        assert_eq!(
            APP_DEVICE_REMOVE_TARGET,
            format!("{APP_TARGET_PREFIX}{DEVICE_REMOVE}")
        );
        assert_eq!(
            APP_WATCH_STATUS_TARGET,
            format!("{APP_TARGET_PREFIX}{WATCH_STATUS}")
        );
        assert_eq!(
            APP_WATCH_SET_TARGET,
            format!("{APP_TARGET_PREFIX}{WATCH_SET}")
        );
        assert_eq!(
            RUNTIME_DEVICE_LIST_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{DEVICE_LIST}")
        );
        assert_eq!(
            RUNTIME_MANAGEMENT_SNAPSHOT_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{MANAGEMENT_SNAPSHOT}")
        );
        assert_eq!(
            RUNTIME_DEVICE_REMOVE_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{DEVICE_REMOVE}")
        );
        assert_eq!(
            RUNTIME_WATCH_STATUS_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{WATCH_STATUS}")
        );
        assert_eq!(
            RUNTIME_WATCH_SET_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{WATCH_SET}")
        );
    }

    #[test]
    fn management_requests_are_strict() {
        let invalid = serde_json::json!({"deviceId": "camera-1", "enabled": true});
        assert!(serde_json::from_value::<WatchStatusRequest>(invalid).is_err());
        let request: WatchSetRequest = serde_json::from_value(serde_json::json!({
            "deviceId": "camera-1",
            "enabled": true
        }))
        .unwrap();
        assert_eq!(request.device_id, "camera-1");
        assert!(request.enabled);
    }

    #[test]
    fn management_conformance_corpus_matches_schema() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schema/camera-plugin-management.v1.schema.json"
        ))
        .unwrap();
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/camera-plugin-management.conformance.json"
        ))
        .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for example in corpus["valid"].as_array().unwrap() {
            assert!(
                validator.is_valid(&example["value"]),
                "valid example was rejected: {}",
                example["name"]
            );
        }
        for example in corpus["invalid"].as_array().unwrap() {
            assert!(
                !validator.is_valid(&example["value"]),
                "invalid example was accepted: {}",
                example["name"]
            );
        }
    }
}
