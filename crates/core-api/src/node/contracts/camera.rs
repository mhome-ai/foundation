use serde::{Deserialize, Serialize};

pub const VERSION: &str = "v1";
pub const NODE_TYPE: &str = "camera";
pub const APP_TARGET_PREFIX: &str = "/app/plugin/camera/";
pub const RUNTIME_TARGET_PREFIX: &str = "/camera/app/";
pub const PROVIDER_LIST: &str = "provider/list";
pub const APP_PROVIDER_LIST_TARGET: &str = "/app/plugin/camera/provider/list";
pub const RUNTIME_PROVIDER_LIST_TARGET: &str = "/camera/app/provider/list";
pub const CONNECTION_LIST: &str = "connection/list";
pub const APP_CONNECTION_LIST_TARGET: &str = "/app/plugin/camera/connection/list";
pub const RUNTIME_CONNECTION_LIST_TARGET: &str = "/camera/app/connection/list";
pub const CONNECTION_DETAIL: &str = "connection/detail";
pub const APP_CONNECTION_DETAIL_TARGET: &str = "/app/plugin/camera/connection/detail";
pub const RUNTIME_CONNECTION_DETAIL_TARGET: &str = "/camera/app/connection/detail";
pub const CONNECTION_DISCONNECT: &str = "connection/disconnect";
pub const APP_CONNECTION_DISCONNECT_TARGET: &str = "/app/plugin/camera/connection/disconnect";
pub const RUNTIME_CONNECTION_DISCONNECT_TARGET: &str = "/camera/app/connection/disconnect";
pub const DEVICE_LIST: &str = "device/list";
pub const MANAGEMENT_SNAPSHOT: &str = "management/snapshot";
pub const DEVICE_SETTINGS_GET: &str = "device/settings/get";
pub const DEVICE_SETTINGS_UPDATE: &str = "device/settings/update";
pub const APP_DEVICE_LIST_TARGET: &str = "/app/plugin/camera/device/list";
pub const APP_MANAGEMENT_SNAPSHOT_TARGET: &str = "/app/plugin/camera/management/snapshot";
pub const APP_DEVICE_SETTINGS_GET_TARGET: &str = "/app/plugin/camera/device/settings/get";
pub const APP_DEVICE_SETTINGS_UPDATE_TARGET: &str = "/app/plugin/camera/device/settings/update";
pub const RUNTIME_DEVICE_LIST_TARGET: &str = "/camera/app/device/list";
pub const RUNTIME_MANAGEMENT_SNAPSHOT_TARGET: &str = "/camera/app/management/snapshot";
pub const RUNTIME_DEVICE_SETTINGS_GET_TARGET: &str = "/camera/app/device/settings/get";
pub const RUNTIME_DEVICE_SETTINGS_UPDATE_TARGET: &str = "/camera/app/device/settings/update";
pub const CONNECTION_CREATE_FLOW: &str = "connection.create";
pub const CONNECTION_RECONFIGURE_FLOW: &str = "connection.reconfigure";
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionIdRequest {
    pub connection_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionDisconnectRequest {
    pub connection_id: String,
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_device_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionDetailResponse {
    pub connection: Option<CameraConnection>,
    pub devices: Vec<CameraDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceIdRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSettingsGetRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSettingsUpdateRequest {
    pub device_id: String,
    pub settings: CameraSettingsPatch,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MonitoringMode {
    Off,
    Auto,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RecordingMode {
    Off,
    Event,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraSettings {
    pub watch_mode: MonitoringMode,
    pub object_detection_mode: MonitoringMode,
    pub person_detection_mode: MonitoringMode,
    pub record_mode: RecordingMode,
    /// Requested pre-event retention in seconds (0..=120), rounded to media fragments.
    pub pre_roll_seconds: u32,
    /// Requested post-event retention in seconds (0..=300), rounded to media fragments.
    pub post_roll_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraSettingsPatch {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub watch_mode: Option<MonitoringMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub object_detection_mode: Option<MonitoringMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub person_detection_mode: Option<MonitoringMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub record_mode: Option<RecordingMode>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub pre_roll_seconds: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "present_setting"
    )]
    pub post_roll_seconds: Option<u32>,
}

fn present_setting<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
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
    pub connection_id: String,
    pub resolution_status: CameraResolutionStatus,
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
pub struct DeviceSettingsResponse {
    pub device_id: String,
    pub revision: u64,
    pub settings: CameraSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CameraResolutionStatus {
    Unresolved,
    Ready,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraConnection {
    pub id: String,
    pub provider: String,
    pub revision: u64,
    pub name: String,
    pub device_count: usize,
    pub last_error: Option<String>,
    pub last_refreshed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManagementSnapshot {
    pub connections: Vec<CameraConnection>,
    pub providers: Vec<ProviderCatalogItem>,
    pub devices: Vec<CameraDevice>,
    pub online_states: Vec<OnlineStateObservation>,
    pub camera_settings: Vec<DeviceSettingsResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceListResponse {
    pub connections: Vec<ConnectionBrief>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionBrief {
    pub id: String,
    pub provider: String,
    pub name: String,
    pub revision: u64,
    pub last_error: Option<String>,
    pub device_count: usize,
    pub devices: Vec<DeviceBrief>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceBrief {
    pub id: String,
    pub name: String,
    pub manufacturer_name: Option<String>,
    pub health: CameraDeviceHealth,
    pub resolution_status: CameraResolutionStatus,
    pub last_error: Option<String>,
    pub online: Option<OnlineBrief>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OnlineBrief {
    pub state: String,
    pub observed_at: Option<i64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionMutationResponse {
    pub connection_id: String,
    pub device_ids: Vec<String>,
    pub changed: bool,
}

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
        assert_eq!(camera["routes"]["deviceSettingsGet"], DEVICE_SETTINGS_GET);
        assert_eq!(
            camera["routes"]["deviceSettingsUpdate"],
            DEVICE_SETTINGS_UPDATE
        );
        assert_eq!(camera["flows"]["connectionCreate"], CONNECTION_CREATE_FLOW);
        assert_eq!(
            camera["flows"]["connectionReconfigure"],
            CONNECTION_RECONFIGURE_FLOW
        );
        assert_eq!(
            camera["routes"]["connectionDisconnect"],
            CONNECTION_DISCONNECT
        );
        assert_eq!(
            APP_DEVICE_LIST_TARGET,
            format!("{APP_TARGET_PREFIX}{DEVICE_LIST}")
        );
        assert_eq!(
            APP_MANAGEMENT_SNAPSHOT_TARGET,
            format!("{APP_TARGET_PREFIX}{MANAGEMENT_SNAPSHOT}")
        );
        assert_eq!(
            APP_CONNECTION_DISCONNECT_TARGET,
            format!("{APP_TARGET_PREFIX}{CONNECTION_DISCONNECT}")
        );
        assert_eq!(
            APP_DEVICE_SETTINGS_GET_TARGET,
            format!("{APP_TARGET_PREFIX}{DEVICE_SETTINGS_GET}")
        );
        assert_eq!(
            APP_DEVICE_SETTINGS_UPDATE_TARGET,
            format!("{APP_TARGET_PREFIX}{DEVICE_SETTINGS_UPDATE}")
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
            RUNTIME_CONNECTION_DISCONNECT_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{CONNECTION_DISCONNECT}")
        );
        assert_eq!(
            RUNTIME_DEVICE_SETTINGS_GET_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{DEVICE_SETTINGS_GET}")
        );
        assert_eq!(
            RUNTIME_DEVICE_SETTINGS_UPDATE_TARGET,
            format!("{RUNTIME_TARGET_PREFIX}{DEVICE_SETTINGS_UPDATE}")
        );
    }

    #[test]
    fn management_requests_are_strict() {
        let invalid = serde_json::json!({"deviceId": "camera-1", "enabled": true});
        assert!(serde_json::from_value::<DeviceSettingsGetRequest>(invalid).is_err());
        let request: DeviceSettingsUpdateRequest = serde_json::from_value(serde_json::json!({
            "deviceId": "camera-1",
            "settings": {"watchMode": "auto"}
        }))
        .unwrap();
        assert_eq!(request.device_id, "camera-1");
        assert_eq!(request.settings.watch_mode, Some(MonitoringMode::Auto));
        assert!(
            serde_json::from_value::<DeviceSettingsUpdateRequest>(serde_json::json!({
                "deviceId":"camera-1", "settings":{"watchMode":null}
            }))
            .is_err()
        );
    }

    #[test]
    fn recording_roll_settings_round_trip_without_changing_other_settings() {
        for (pre, post) in [(0, 0), (8, 15), (120, 300)] {
            let value = serde_json::json!({
                "deviceId": "camera-1",
                "settings": {"preRollSeconds": pre, "postRollSeconds": post}
            });
            let request: DeviceSettingsUpdateRequest =
                serde_json::from_value(value.clone()).unwrap();
            assert_eq!(request.settings.pre_roll_seconds, Some(pre));
            assert_eq!(request.settings.post_roll_seconds, Some(post));
            assert_eq!(request.settings.record_mode, None);
            assert_eq!(serde_json::to_value(request).unwrap(), value);
        }
        for field in ["preRollSeconds", "postRollSeconds"] {
            let value = serde_json::json!({
                "deviceId": "camera-1", "settings": {field: 15}
            });
            let request: DeviceSettingsUpdateRequest =
                serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(request).unwrap(), value);
            for invalid in [
                serde_json::Value::Null,
                serde_json::json!(-1),
                serde_json::json!(1.5),
                serde_json::json!("15"),
                serde_json::json!(u64::from(u32::MAX) + 1),
            ] {
                assert!(
                    serde_json::from_value::<DeviceSettingsUpdateRequest>(serde_json::json!({
                        "deviceId": "camera-1", "settings": {field: invalid}
                    }))
                    .is_err()
                );
            }
        }
    }

    #[test]
    fn settings_snapshot_requires_explicit_recording_roll_values() {
        let value = serde_json::json!({
            "deviceId": "camera-1", "revision": 1,
            "settings": {
                "watchMode": "off", "objectDetectionMode": "auto",
                "personDetectionMode": "off", "recordMode": "event",
                "preRollSeconds": 8, "postRollSeconds": 15
            }
        });
        let response: DeviceSettingsResponse = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(response.settings.pre_roll_seconds, 8);
        assert_eq!(response.settings.post_roll_seconds, 15);
        assert_eq!(serde_json::to_value(response).unwrap(), value);
        for field in ["preRollSeconds", "postRollSeconds"] {
            let mut missing = value.clone();
            missing["settings"].as_object_mut().unwrap().remove(field);
            assert!(serde_json::from_value::<DeviceSettingsResponse>(missing).is_err());
            let mut null = value.clone();
            null["settings"][field] = serde_json::Value::Null;
            assert!(serde_json::from_value::<DeviceSettingsResponse>(null).is_err());
        }
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
