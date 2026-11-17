//! Core-native existing-network Wave admission. Keys/grants never use this DTO.
use serde::{Deserialize, Serialize};
pub const DISCOVER: &str = "/app/wave/discover";
pub const LIST: &str = "/app/wave/list";
pub const REFRESH: &str = "/app/wave/refresh";
pub const REMOVE: &str = "/app/wave/remove";
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}
use crate::pod::DiscoveryTransport;
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoverInput {
    pub transport: DiscoveryTransport,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceInput {
    pub device_id: String,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveInput {
    pub device_id: String,
    #[serde(default)]
    pub forget_after_device_reset: bool,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RemoveResult {
    pub device_id: String,
    pub grant_state: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub candidate_id: String,
    pub device_id: String,
    pub proof: String,
    pub purpose: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Binding {
    pub device_id: String,
    pub grant_state: String,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionResult {
    pub device_id: String,
    pub grant_state: String,
    pub inventory_state: String,
    pub entity_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn wave_management_is_hub_routed_and_requires_manage() {
        for target in [DISCOVER, LIST, REFRESH, REMOVE] {
            assert_eq!(
                crate::authorization::required(target),
                Some(crate::authorization::OperationPolicy::SpaceManage)
            );
            assert!(!crate::authorization::allows_service(target));
        }
        assert_eq!(crate::authorization::required("/app/wave/pair"), None);
    }
    #[test]
    fn wave_inputs_cannot_supply_authority_routes_or_credentials() {
        for key in ["spaceId", "hubId", "address", "grant", "privateKey"] {
            assert!(serde_json::from_value::<DiscoverInput>(
                json!({"transport":"lan",key:"foreign"})
            )
            .is_err());
        }
        assert!(serde_json::from_value::<EmptyInput>(json!({"address":"1.2.3.4"})).is_err());
    }

    #[test]
    fn wave_discovery_requires_exactly_one_explicit_transport() {
        for (name, expected) in [
            ("lan", DiscoveryTransport::Lan),
            ("ble", DiscoveryTransport::Ble),
        ] {
            let input: DiscoverInput = serde_json::from_value(json!({"transport":name})).unwrap();
            assert_eq!(input.transport, expected);
            assert_eq!(
                serde_json::to_value(input).unwrap(),
                json!({"transport":name})
            );
        }
        for input in [
            json!({}),
            json!({"transport":"all"}),
            json!({"transport":["lan","ble"]}),
            json!({"transport":"lan","address":"192.0.2.1"}),
        ] {
            assert!(serde_json::from_value::<DiscoverInput>(input).is_err());
        }
    }
}
