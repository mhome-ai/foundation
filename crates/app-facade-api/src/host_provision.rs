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

pub const PROVISION_CHANGED_EVENT: &str = "/local/host/provision/changed";

pub const LOCAL_TARGETS: &[&str] = &[
    PROVISION_START_TARGET,
    PROVISION_CODE_TARGET,
    PROVISION_WIFI_TARGET,
    PROVISION_WIFI_SCAN_TARGET,
    PROVISION_STATUS_TARGET,
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
    Completed,
    Failed,
    TimedOut,
}

impl ProvisionState {
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::TimedOut)
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
    /// IPv4 LAN addresses reported by the Host once it joined; may be empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub addresses: Vec<String>,
    /// One-time token for the first claim of this Host, from `finish`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claim_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CommissionError>,
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
                    "only failed, timed_out and retryable code or Wi-Fi states carry an error",
                );
            }
        }
        if let Some(code) = self.error.as_ref().map(|error| error.code) {
            if matches!(
                code,
                CommissionErrorCode::IssueFailed
                    | CommissionErrorCode::DeliveryFailed
                    | CommissionErrorCode::ActivationFailed
            ) {
                return Err("credential error codes do not apply to Host provisioning");
            }
        }
        if self.state == ProvisionState::Completed && self.host.is_none() {
            return Err("completed sessions carry the Host information");
        }
        match self.claim_token.as_deref() {
            Some(_) if self.state != ProvisionState::Completed => {
                return Err("only completed sessions carry a claim token");
            }
            Some(token) if !is_claim_token(token) => {
                return Err("claim tokens are 43 base64url characters");
            }
            _ => {}
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
