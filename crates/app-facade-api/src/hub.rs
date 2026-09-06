use serde::{Deserialize, Serialize};

pub const GET_TARGET: &str = "/app/hub/get";
pub const REMOVE_TARGET: &str = "/app/hub/remove";
pub const CACHE_CLEAR_TARGET: &str = "/app/hub/cache/clear";

pub const LOCAL_GET_TARGET: &str = "/local/hub/get";
pub const LOCAL_CANDIDATES_TARGET: &str = "/local/hub/candidates";
pub const LOCAL_ADD_START_TARGET: &str = "/local/hub/add/start";
pub const LOCAL_ADD_CONFIRM_TARGET: &str = "/local/hub/add/confirm";
pub const LOCAL_ADD_STATUS_TARGET: &str = "/local/hub/add/status";
pub const LOCAL_ADD_CANCEL_TARGET: &str = "/local/hub/add/cancel";

pub const APP_TARGETS: &[&str] = &[GET_TARGET, REMOVE_TARGET, CACHE_CLEAR_TARGET];
pub const LOCAL_TARGETS: &[&str] = &[
    LOCAL_GET_TARGET,
    LOCAL_CANDIDATES_TARGET,
    LOCAL_ADD_START_TARGET,
    LOCAL_ADD_CONFIRM_TARGET,
    LOCAL_ADD_STATUS_TARGET,
    LOCAL_ADD_CANCEL_TARGET,
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubGetRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_hub_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubState {
    pub scope_id: String,
    pub configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub: Option<HubView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<HubOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation: Option<HubRevocation>,
}

impl HubState {
    pub fn validate(&self, known_hub_id: Option<&str>) -> Result<(), &'static str> {
        if self.configured != self.hub.is_some() {
            return Err("configured must exactly match whether a published Hub is present");
        }
        if let Some(revocation) = &self.revocation {
            if known_hub_id != Some(revocation.hub_id.as_str()) {
                return Err("revocation must exactly match knownHubId");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubView {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub hub_type: String,
    pub host_id: String,
    pub is_test: bool,
    pub status: String,
    pub cloud: HubCloudView,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubCloudView {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_connected_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub firmware_version: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HubOperationKind {
    Setup,
    Remove,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HubOperationState {
    Running,
    Failed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HubOperationPhase {
    WaitingForHub,
    PreparingData,
    SyncingData,
    BackingUpData,
    FencingCloud,
    ClearingLocalData,
    Finalizing,
    CleaningUp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubOperation {
    pub kind: HubOperationKind,
    pub state: HubOperationState,
    pub phase: HubOperationPhase,
    pub started_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<HubOperationFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubOperationFailure {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubRevocation {
    pub hub_id: String,
    pub revocation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubRemoveRequest {
    pub hub_id: String,
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubRemoveResponse {
    pub accepted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<HubOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubCacheClearRequest {
    pub hub_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalHubState {
    pub scope_id: String,
    pub configured: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub: Option<LocalHubView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<HubOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revocation: Option<HubRevocation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing: Option<HubPairingOperation>,
}

impl LocalHubState {
    pub fn validate(&self, known_hub_id: Option<&str>) -> Result<(), &'static str> {
        if self.configured != self.hub.is_some() {
            return Err("configured must exactly match whether a published Hub is present");
        }
        if let Some(revocation) = &self.revocation {
            if known_hub_id != Some(revocation.hub_id.as_str()) {
                return Err("revocation must exactly match knownHubId");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalHubView {
    #[serde(flatten)]
    pub cloud: HubView,
    pub local: LocalHubRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalHubRuntime {
    pub discovered: bool,
    pub connected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubPairingOperation {
    pub session_id: String,
    pub state: String,
    pub phase: String,
    pub started_at: i64,
    pub expires_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<HubOperationFailure>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubCandidatesResponse {
    pub scope_id: String,
    #[serde(default)]
    pub candidates: Vec<HubCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubCandidate {
    pub host_id: String,
    pub host_name: String,
    #[serde(rename = "type")]
    pub hub_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubEndpoint {
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddStartRequest {
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<HubEndpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddStartResponse {
    pub scope_id: String,
    pub session_id: String,
    pub candidate: HubCandidate,
    pub pairing_code: String,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddConfirmResponse {
    pub scope_id: String,
    pub session_id: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddStatusResponse {
    pub scope_id: String,
    pub session_id: String,
    pub complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pairing: Option<HubPairingOperation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<HubOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HubAddCancelResponse {
    pub scope_id: String,
    pub session_id: String,
    pub cancelled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_state_enforces_published_and_exact_revocation_invariants() {
        let state = HubState {
            scope_id: "scope-1".into(),
            configured: false,
            revocation: Some(HubRevocation {
                hub_id: "hub-old".into(),
                revocation_id: "revoke-1".into(),
            }),
            ..HubState::default()
        };
        assert!(state.validate(Some("hub-old")).is_ok());
        assert!(state.validate(Some("hub-other")).is_err());

        let mut invalid = state;
        invalid.configured = true;
        assert!(invalid.validate(Some("hub-old")).is_err());
    }

    #[test]
    fn local_targets_are_disjoint_and_never_public_app_targets() {
        assert!(APP_TARGETS.iter().all(|target| target.starts_with("/app/")));
        assert!(LOCAL_TARGETS
            .iter()
            .all(|target| target.starts_with("/local/")));
        assert!(APP_TARGETS
            .iter()
            .all(|target| !LOCAL_TARGETS.contains(target)));
    }
}
