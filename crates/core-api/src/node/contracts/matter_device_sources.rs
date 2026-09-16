//! Read-only Matter provenance projected by the Node, without Fabric internals.
use serde::{Deserialize, Serialize};
pub const MATTER_DEVICE_SOURCES_TARGET: &str = "/matter/device/sources";
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatterDeviceSourcesRequest {}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MatterSourceAvailability {
    Available,
    Unavailable,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatterDeviceSourcesSnapshot {
    pub availability: MatterSourceAvailability,
    pub bridges: Vec<MatterSourceBridge>,
    pub devices: Vec<MatterDeviceSource>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatterSourceBridge {
    pub node_id: String,
    pub display_name: String,
    pub reachable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatterDeviceSource {
    pub node_id: String,
    pub availability: MatterSourceAvailability,
    pub bridge_node_id: Option<String>,
}
impl MatterDeviceSourcesSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        use MatterSourceAvailability::*;
        if self.availability == Unavailable
            && (!self.bridges.is_empty() || !self.devices.is_empty())
        {
            return Err("Unavailable Matter inventory must be empty".into());
        }
        let mut bridges = std::collections::HashSet::new();
        for bridge in &self.bridges {
            if bridge.node_id.trim().is_empty()
                || bridge.display_name.trim().is_empty()
                || !bridges.insert(bridge.node_id.as_str())
            {
                return Err("Invalid or duplicate Matter bridge".into());
            }
        }
        let mut devices = std::collections::HashSet::new();
        for device in &self.devices {
            if device.node_id.trim().is_empty() || !devices.insert(device.node_id.as_str()) {
                return Err("Invalid or duplicate Matter source".into());
            }
            if let Some(bridge) = &device.bridge_node_id {
                if device.availability != Available
                    || !bridges.contains(bridge.as_str())
                    || bridge != &device.node_id
                {
                    return Err("Invalid Matter bridge reference".into());
                }
            }
        }
        for bridge in &self.bridges {
            if !self.devices.iter().any(|d| {
                d.node_id == bridge.node_id
                    && d.bridge_node_id.as_deref() == Some(bridge.node_id.as_str())
            }) {
                return Err("Matter bridge has no source mapping".into());
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
    fn validates_direct_bridged_and_unknown_sources() {
        let value = json!({"availability":"available","bridges":[{"nodeId":"1","displayName":"Bridge","reachable":false}],"devices":[{"nodeId":"1","availability":"available","bridgeNodeId":"1"},{"nodeId":"2","availability":"available","bridgeNodeId":null},{"nodeId":"3","availability":"unavailable","bridgeNodeId":null}]});
        let snapshot: MatterDeviceSourcesSnapshot = serde_json::from_value(value.clone()).unwrap();
        assert!(snapshot.validate().is_ok());
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), value);
        let mut invalid = snapshot.clone();
        invalid.devices[0].bridge_node_id = Some("missing".into());
        assert!(invalid.validate().is_err());
        let mut invalid = snapshot.clone();
        invalid.availability = MatterSourceAvailability::Unavailable;
        assert!(invalid.validate().is_err());
        let mut invalid = snapshot.clone();
        invalid.devices[0].availability = MatterSourceAvailability::Unavailable;
        assert!(invalid.validate().is_err());
        let mut invalid = snapshot.clone();
        invalid.devices.push(invalid.devices[0].clone());
        assert!(invalid.validate().is_err());
        let mut invalid = value;
        invalid["bridges"][0]["endpoints"] = json!([]);
        assert!(serde_json::from_value::<MatterDeviceSourcesSnapshot>(invalid).is_err());
        assert!(
            serde_json::from_value::<MatterDeviceSourcesRequest>(json!({"discover":true})).is_err()
        );
    }
}
