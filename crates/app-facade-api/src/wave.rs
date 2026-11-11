//! Core-native existing-network Wave admission. Keys/grants never use this DTO.
use serde::{Deserialize, Serialize};
pub const DISCOVER: &str = "/app/wave/discover";
pub const PAIR: &str = "/app/wave/pair";
pub const LIST: &str = "/app/wave/list";
pub const REFRESH: &str = "/app/wave/refresh";
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct EmptyInput {}
// Deliberately no Debug: this is a private direct App interaction.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PairInput {
    pub candidate_id: String,
    pub pin: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceInput {
    pub device_id: String,
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
        for target in [DISCOVER, PAIR, LIST, REFRESH] {
            assert_eq!(
                crate::authorization::required(target),
                Some(crate::authorization::OperationPolicy::SpaceManage)
            );
            assert!(!crate::authorization::allows_service(target));
        }
        // No raw PIN/candidate payload enters household history projection.
        assert!(!crate::recording::selected(PAIR));
    }
    #[test]
    fn wave_inputs_cannot_supply_authority_routes_or_credentials() {
        assert!(
            serde_json::from_value::<PairInput>(json!({"candidateId":"hint","pin":"001234"}))
                .is_ok()
        );
        for key in ["spaceId", "hubId", "address", "grant", "privateKey"] {
            assert!(serde_json::from_value::<PairInput>(
                json!({"candidateId":"hint",key:"foreign"})
            )
            .is_err());
        }
        assert!(serde_json::from_value::<EmptyInput>(json!({"address":"1.2.3.4"})).is_err());
    }
}
