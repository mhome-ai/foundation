//! Native-Client pod discovery and commissioning. See
//! `contract/pod-commissioning-v1.md`.
use serde::{Deserialize, Serialize};

pub const DISCOVERY_START_TARGET: &str = "/local/pod/discovery/start";
pub const DISCOVERY_RENEW_TARGET: &str = "/local/pod/discovery/renew";
pub const DISCOVERY_STOP_TARGET: &str = "/local/pod/discovery/stop";
pub const DISCOVERY_LIST_TARGET: &str = "/local/pod/discovery/list";
pub const COMMISSION_START_TARGET: &str = "/local/pod/commission/start";
pub const COMMISSION_CODE_TARGET: &str = "/local/pod/commission/code";
pub const COMMISSION_WIFI_TARGET: &str = "/local/pod/commission/wifi";
pub const COMMISSION_WIFI_SCAN_TARGET: &str = "/local/pod/commission/wifi/scan";
pub const COMMISSION_AUTHORIZE_TARGET: &str = "/local/pod/commission/authorize";
pub const COMMISSION_CANCEL_TARGET: &str = "/local/pod/commission/cancel";
pub const COMMISSION_STATUS_TARGET: &str = "/local/pod/commission/status";

pub const DISCOVERY_CHANGED_EVENT: &str = "/local/pod/discovery/changed";
pub const COMMISSION_CHANGED_EVENT: &str = "/local/pod/commission/changed";

pub const LOCAL_TARGETS: &[&str] = &[
    DISCOVERY_START_TARGET,
    DISCOVERY_RENEW_TARGET,
    DISCOVERY_STOP_TARGET,
    DISCOVERY_LIST_TARGET,
    COMMISSION_START_TARGET,
    COMMISSION_CODE_TARGET,
    COMMISSION_WIFI_TARGET,
    COMMISSION_WIFI_SCAN_TARGET,
    COMMISSION_AUTHORIZE_TARGET,
    COMMISSION_CANCEL_TARGET,
    COMMISSION_STATUS_TARGET,
];
pub const EVENT_TARGETS: &[&str] = &[DISCOVERY_CHANGED_EVENT, COMMISSION_CHANGED_EVENT];

/// Shared by every commissionable MeowLink device kind.
pub const BLE_SERVICE_UUID: &str = "a17a7ad5-9b2f-4010-9cf0-aa15ac54c40e";
/// Bluetooth SIG test value; replace with an assigned company ID before release.
pub const BLE_COMPANY_ID: u16 = 0xFFFF;
pub const BLE_MANUFACTURER_MAGIC: [u8; 2] = *b"MW";
pub const BLE_ADVERTISING_VERSION: u8 = 1;
pub const BLE_KIND_POD: u8 = 1;
pub const BLE_KIND_HOST: u8 = 2;
pub const BLE_FLAG_COMMISSIONABLE: u8 = 0x01;
pub const BLE_FLAG_WIFI_CONFIGURED: u8 = 0x02;

pub const DISCOVERY_LEASE_MS: i64 = 30_000;
pub const CANDIDATE_TTL_MS: i64 = 10_000;
pub const CUSTOM_ENDPOINT_MAX_BYTES: usize = 480;
pub const SRP_USERNAME: &str = "meow";
pub const POD_INFO_ENDPOINT: &str = "pod-info";
pub const POD_CREDENTIAL_ENDPOINT: &str = "pod-credential";

/// Manufacturer-data payload after the 2-byte company ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BleAdvertisement {
    pub kind: u8,
    pub flags: u8,
    pub short_id: [u8; 4],
}

impl BleAdvertisement {
    pub const LEN: usize = 9;

    pub fn encode(&self) -> [u8; Self::LEN] {
        let [m0, m1] = BLE_MANUFACTURER_MAGIC;
        let [s0, s1, s2, s3] = self.short_id;
        [
            m0,
            m1,
            BLE_ADVERTISING_VERSION,
            self.kind,
            self.flags,
            s0,
            s1,
            s2,
            s3,
        ]
    }

    /// `data` excludes the company ID. Unknown magic or version yields `None`.
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < Self::LEN
            || data[..2] != BLE_MANUFACTURER_MAGIC
            || data[2] != BLE_ADVERTISING_VERSION
        {
            return None;
        }
        Some(Self {
            kind: data[3],
            flags: data[4],
            short_id: [data[5], data[6], data[7], data[8]],
        })
    }

    pub fn commissionable(&self) -> bool {
        self.flags & BLE_FLAG_COMMISSIONABLE != 0
    }

    pub fn wifi_configured(&self) -> bool {
        self.flags & BLE_FLAG_WIFI_CONFIGURED != 0
    }

    pub fn short_id_hex(&self) -> String {
        self.short_id
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LeaseRequest {
    pub lease_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoveryLease {
    pub lease_id: String,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AdapterState {
    Ready,
    PoweredOff,
    Denied,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdapterStatus {
    pub state: AdapterState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PodCandidate {
    pub candidate_id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub short_id: String,
    pub rssi: i16,
    pub commissionable: bool,
    pub wifi_configured: bool,
    pub last_seen_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiscoverySnapshot {
    pub revision: u64,
    pub scanning: bool,
    pub adapter: AdapterStatus,
    pub candidates: Vec<PodCandidate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionStartRequest {
    pub candidate_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionCodeRequest {
    pub session_id: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionWifiRequest {
    pub session_id: String,
    pub ssid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionSessionRequest {
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionAuthorizeRequest {
    pub session_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommissionState {
    Connecting,
    AwaitingCode,
    Securing,
    ReadingInfo,
    AwaitingWifi,
    JoiningWifi,
    AwaitingAuthorization,
    Issuing,
    Delivering,
    Activating,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl CommissionState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommissionErrorCode {
    BluetoothUnavailable,
    DeviceBusy,
    Disconnected,
    CodeRejected,
    CodeLocked,
    WifiAuthFailed,
    WifiNotFound,
    WifiFailed,
    IssueFailed,
    DeliveryFailed,
    ActivationFailed,
    SessionTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionError {
    pub code: CommissionErrorCode,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PodDeviceInfo {
    pub device_id: String,
    pub model: String,
    pub firmware_version: String,
    pub wifi_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WifiNetwork {
    pub ssid: String,
    pub rssi: i16,
    pub secured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationScope {
    pub scope_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationPrompt {
    pub user_id: String,
    pub user_name: String,
    pub scopes: Vec<AuthorizationScope>,
    pub default_scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionSession {
    pub session_id: String,
    pub revision: u64,
    pub state: CommissionState,
    pub candidate: PodCandidate,
    pub expires_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<PodDeviceInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub networks: Vec<WifiNetwork>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authorization: Option<AuthorizationPrompt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pod_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<CommissionError>,
}

impl CommissionSession {
    pub fn validate(&self) -> Result<(), &'static str> {
        match (self.state, self.error.as_ref().map(|error| error.code)) {
            (CommissionState::Failed | CommissionState::TimedOut, None) => {
                return Err("failed and timed_out sessions carry an error");
            }
            (CommissionState::Failed | CommissionState::TimedOut, Some(_))
            | (_, None)
            | (CommissionState::AwaitingCode, Some(CommissionErrorCode::CodeRejected))
            | (
                CommissionState::AwaitingWifi,
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
        if self.state == CommissionState::AwaitingAuthorization && self.authorization.is_none() {
            return Err("awaiting_authorization requires an authorization prompt");
        }
        if self.state == CommissionState::Completed && self.pod_id.is_none() {
            return Err("completed sessions carry the issued podId");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommissionStatus {
    pub session: Option<CommissionSession>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertisement_round_trips_and_rejects_unknown_versions() {
        let ad = BleAdvertisement {
            kind: BLE_KIND_POD,
            flags: BLE_FLAG_COMMISSIONABLE,
            short_id: [0x1a, 0x2b, 0x3c, 0x4d],
        };
        let bytes = ad.encode();
        assert_eq!(BleAdvertisement::decode(&bytes), Some(ad));
        assert_eq!(ad.short_id_hex(), "1A2B3C4D");
        assert!(ad.commissionable() && !ad.wifi_configured());

        let mut other = bytes;
        other[2] = 2;
        assert_eq!(BleAdvertisement::decode(&other), None);
        assert_eq!(BleAdvertisement::decode(&bytes[..8]), None);
    }
}
