//! Native-Client provisioning of embedded Hosts over BLE. See
//! `contract/host-provisioning-v1.md`. Discovery is shared with pods
//! (`/local/pod/discovery/*`, candidates of kind `host`).
use serde::{Deserialize, Serialize};

use crate::pod::{CommissionError, CommissionErrorCode, PodCandidate, WifiNetwork};

pub const PROVISION_START_TARGET: &str = "/local/host/provision/start";
pub const PROVISION_CODE_TARGET: &str = "/local/host/provision/code";
pub const PROVISION_WIFI_TARGET: &str = "/local/host/provision/wifi";
pub const PROVISION_WIFI_SCAN_TARGET: &str = "/local/host/provision/wifi/scan";
pub const PROVISION_STATUS_TARGET: &str = "/local/host/provision/status";
pub const PROVISION_AUTHORIZE_TARGET: &str = "/local/host/provision/authorize";
pub const PROVISION_CANCEL_TARGET: &str = "/local/host/provision/cancel";

pub const PROVISION_CHANGED_EVENT: &str = "/local/host/provision/changed";

pub const LOCAL_TARGETS: &[&str] = &[
    PROVISION_START_TARGET,
    PROVISION_CODE_TARGET,
    PROVISION_WIFI_TARGET,
    PROVISION_WIFI_SCAN_TARGET,
    PROVISION_STATUS_TARGET,
    PROVISION_AUTHORIZE_TARGET,
    PROVISION_CANCEL_TARGET,
];
pub const EVENT_TARGETS: &[&str] = &[PROVISION_CHANGED_EVENT];

/// App info capability in the Host's `proto-ver` response.
pub const APP_CAPABILITY: &str = "host";
pub use crate::pod::{APP_INFO_LABEL, APP_INFO_LOCKED, APP_INFO_LOCKED_FOR_MS};
/// Largest request and encrypted response value; scan results are paged to fit.
pub const MAX_ATTRIBUTE_BYTES: usize = 512;
/// The Host answers `prov-scan` start within this long.
pub const SCAN_TIMEOUT_MS: i64 = 12_000;
pub const HOST_INFO_ENDPOINT: &str = "host-info";
pub const HOST_ENDPOINT_UUIDS: &[(&str, u16)] = &[(HOST_INFO_ENDPOINT, 0xFF54)];
/// The Host keeps BLE open this long after a successful join without `finish`.
pub const FINISH_GRACE_MS: i64 = 30_000;
/// Handshakes are refused this long after each code rotation caused by failed
/// handshakes, doubling per consecutive rotation up to `CODE_LOCKOUT_MAX_MS`.
pub const CODE_LOCKOUT_MS: i64 = 30_000;
pub const CODE_LOCKOUT_MAX_MS: i64 = 600_000;
/// Unpadded base64url of 32 random bytes.
pub const CLAIM_TOKEN_LEN: usize = 43;

pub fn is_claim_token(token: &str) -> bool {
    token.len() == CLAIM_TOKEN_LEN
        && token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// Requests reuse the pod request shapes: start takes `candidateId`, code takes
/// `sessionId` and `code`, Wi-Fi takes `sessionId`, `ssid` and `password`, the
/// rest take `sessionId` (status takes `{}`).
pub use crate::pod::{
    CommissionCodeRequest as ProvisionCodeRequest,
    CommissionSessionRequest as ProvisionSessionRequest,
    CommissionStartRequest as ProvisionStartRequest, CommissionWifiRequest as ProvisionWifiRequest,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProvisionState {
    Connecting,
    AwaitingCode,
    Securing,
    ReadingInfo,
    AwaitingWifi,
    JoiningWifi,
    AwaitingAuthorization,
    Authorizing,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    TimedOut,
}

impl ProvisionState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::TimedOut | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostDeviceInfo {
    pub host_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,
    pub firmware_version: String,
    pub fingerprint: String,
    pub wifi_configured: bool,
    pub authorization_mode: core_api::host::access::HostMode,
    pub onboarding_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionSession {
    pub session_id: String,
    pub revision: u64,
    pub state: ProvisionState,
    pub candidate: PodCandidate,
    pub expires_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<HostDeviceInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<WifiNetwork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<crate::pod::AuthorizationPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enrollment: Option<crate::pod::EnrollmentTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<ProvisionResult>,
    /// IPv4 LAN addresses reported by the Host once it joined; may be empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CommissionError>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionResult {
    pub host_id: String,
    pub wifi_configured: bool,
    pub commissioned: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
}

impl ProvisionSession {
    pub fn validate(&self) -> Result<(), &'static str> {
        match (self.state, self.error.as_ref().map(|error| error.code)) {
            (ProvisionState::Failed | ProvisionState::TimedOut, None) => {
                return Err("failed and timed_out sessions carry an error");
            }
            (ProvisionState::Failed | ProvisionState::TimedOut, Some(_))
            | (_, None)
            | (ProvisionState::AwaitingCode, Some(CommissionErrorCode::CodeRejected))
            | (ProvisionState::AwaitingAuthorization, Some(CommissionErrorCode::IssueFailed))
            | (
                ProvisionState::AwaitingWifi,
                Some(
                    CommissionErrorCode::WifiAuthFailed
                    | CommissionErrorCode::WifiNotFound
                    | CommissionErrorCode::WifiFailed,
                ),
            ) => {}
            _ => {
                return Err(
                    "only failed, timed_out and retryable interaction states carry an error",
                );
            }
        }
        if let Some(code) = self.error.as_ref().map(|error| error.code) {
            if matches!(
                code,
                CommissionErrorCode::DeliveryFailed | CommissionErrorCode::ActivationFailed
            ) {
                return Err("credential error codes do not apply to Host provisioning");
            }
        }
        match (&self.result, self.state) {
            (Some(result), ProvisionState::Completed)
                if self.host.as_ref().is_some_and(|host| {
                    host.host_id == result.host_id && result.wifi_configured && result.commissioned
                }) => {}
            (None, ProvisionState::Completed) => {
                return Err("completed sessions carry their operation result")
            }
            (Some(_), _) => {
                return Err("only completed sessions carry a matching successful operation result")
            }
            (None, _) => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionStatus {
    pub session: Option<ProvisionSession>,
}

/// `host-info` `{"op":"finish"}` response. `claimToken` is present after a
/// successful join.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostFinishResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_token: Option<String>,
}
