use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::V1;
pub const PLUGIN_TYPE: &str = "audiobridge";
pub const RUNTIME_TARGET_PREFIX: &str = "/audiobridge/app/";
pub const MANAGEMENT_SNAPSHOT: &str = "management/snapshot";
pub const ENDPOINTS_REFRESH: &str = "endpoints/refresh";
pub const DEVICE_ADOPT: &str = "device/adopt";
pub const DEVICE_UNADOPT: &str = "device/unadopt";
pub const DEVICE_TEST: &str = "device/test";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeDevice {
    pub device_id: String,
    pub reported_name: String,
    pub transport: String,
    pub adopted: bool,
    pub online: bool,
    pub output_route_ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeCandidate {
    pub candidate_id: String,
    pub reported_name: String,
    pub transport: String,
    pub adoptable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagementSnapshot {
    pub inventory_revision: u64,
    #[serde(default)]
    pub devices: Vec<AudioBridgeDevice>,
    #[serde(default)]
    pub candidates: Vec<AudioBridgeCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagementSnapshotResponse {
    pub ok: bool,
    pub snapshot: ManagementSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionResponse {
    pub ok: bool,
    pub changed: bool,
    pub device: AudioBridgeDevice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_contract_does_not_expose_platform_identity() {
        let value = serde_json::to_value(AudioBridgeDevice {
            device_id: "device-1".to_string(),
            reported_name: "Living Room Speaker".to_string(),
            transport: "bluetooth".to_string(),
            adopted: true,
            online: true,
            output_route_ready: true,
        })
        .expect("serialize device");
        assert!(value.get("platformKey").is_none());
        assert!(value.get("endpointId").is_none());
    }
}
