//! Safe Camera provenance. Never carries source configuration or credentials.
use serde::{Deserialize, Serialize};

pub const CAMERA_TOPOLOGY_TARGET: &str = "/camera/device/topology/sources";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraTopologyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraTopologySnapshot {
    pub providers: Vec<CameraTopologyProvider>,
    pub devices: Vec<CameraTopologyDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraTopologyProvider {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CameraTopologyDevice {
    pub device_id: String,
    pub provider_id: String,
}

impl CameraTopologySnapshot {
    pub fn validate(&self) -> Result<(), String> {
        let mut providers = std::collections::HashSet::new();
        let mut devices = std::collections::HashSet::new();
        for provider in &self.providers {
            if provider.id.trim().is_empty()
                || provider.display_name.trim().is_empty()
                || !providers.insert(provider.id.as_str())
            {
                return Err("Invalid or duplicate Camera provider".into());
            }
        }
        for device in &self.devices {
            if device.device_id.trim().is_empty()
                || !devices.insert(device.device_id.as_str())
                || !providers.contains(device.provider_id.as_str())
            {
                return Err("Invalid Camera device or provider reference".into());
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn topology_wire_contract_and_identity_constraints() {
        let value = json!({"providers":[{"id":"rtsp","displayName":"RTSP"}],
            "devices":[{"deviceId":"camera-1","providerId":"rtsp"}]});
        let snapshot: CameraTopologySnapshot = serde_json::from_value(value.clone()).unwrap();
        assert!(snapshot.validate().is_ok());
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), value);
        let mut duplicate = snapshot.clone();
        duplicate.devices.push(snapshot.devices[0].clone());
        assert!(duplicate.validate().is_err());
        let mut dangling = snapshot.clone();
        dangling.devices[0].provider_id = "absent".into();
        assert!(dangling.validate().is_err());
        let mut leaked = value;
        leaked["devices"][0]["password"] = json!("secret");
        assert!(serde_json::from_value::<CameraTopologySnapshot>(leaked).is_err());
    }
    #[test]
    fn topology_request_and_response_are_strict() {
        assert!(serde_json::from_value::<CameraTopologyRequest>(json!({})).is_ok());
        assert!(serde_json::from_value::<CameraTopologyRequest>(json!({"extra":true})).is_err());
        assert!(serde_json::from_value::<CameraTopologySnapshot>(json!({"providers":[]})).is_err());
    }
}
