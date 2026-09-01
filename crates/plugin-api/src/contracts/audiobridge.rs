use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::V1;
pub const NODE_TYPE: &str = "audiobridge";
pub const RUNTIME_TARGET_PREFIX: &str = "/audiobridge/app/";
pub const DEVICE_SNAPSHOT: &str = "device/snapshot";
pub const DEVICE_REFRESH: &str = "device/refresh";
pub const DEVICE_ADOPT: &str = "device/adopt";
pub const DEVICE_UNPAIR: &str = "device/unpair";
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
    pub display_name: String,
    pub transport: String,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeCandidate {
    pub device_id: String,
    pub display_name: String,
    pub transport: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    #[serde(default)]
    pub devices: Vec<AudioBridgeDevice>,
    #[serde(default)]
    pub candidates: Vec<AudioBridgeCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptionResponse {
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
            display_name: "Living Room Speaker".to_string(),
            transport: "bluetooth".to_string(),
            online: true,
        })
        .expect("serialize device");
        assert!(value.get("platformKey").is_none());
        assert!(value.get("endpointId").is_none());
    }

    #[test]
    fn device_snapshot_is_the_direct_response_payload() {
        let value = serde_json::to_value(DeviceSnapshot {
            devices: Vec::new(),
            candidates: Vec::new(),
        })
        .expect("serialize device snapshot");
        assert!(value.get("devices").is_some());
        assert!(value.get("candidates").is_some());
        assert!(value.get("snapshot").is_none());
    }
}
