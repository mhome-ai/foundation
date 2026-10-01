//! OS authorization for one computer's Host and the services it runs; independent
//! of any Space.
//!
//! Services declare requirements and report observations from the processes that
//! actually access the resource. An executing process is not necessarily a
//! separate OS authorization identity: helpers can share a responsible app.
//! Host reports one row per component and permission. Reading a report must never request authorization.
//! A previous access probe is historical evidence, not a current OS setting.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type PlatformPermissionRequirements = BTreeMap<String, Vec<PermissionRequirement>>;

/// Automation is authorized per target application, not as one global switch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(tag = "id", rename_all = "camelCase", deny_unknown_fields)]
pub enum PermissionKey {
    LocalNetwork {},
    Bluetooth {},
    Microphone {},
    Reminders {},
    Automation {
        #[serde(rename = "targetBundleId")]
        target_bundle_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionRequirement {
    pub permission: PermissionKey,
    /// Human-readable affected capability; denial need not disable the service.
    pub feature: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    Unknown,
    NotDetermined,
    Granted,
    /// This platform has no per-application consent for this resource. This says
    /// nothing about device, account, session, sandbox or system-policy access.
    NotRequired,
    Denied,
    Restricted,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionEvidence {
    /// A passive OS authorization query, not hardware availability.
    System,
    /// Platform semantics, e.g. Linux has no TCC consent for Bluetooth.
    /// This is not evidence that an operation or device is available.
    Platform,
    /// A previous explicit request/probe; observedAtMs is mandatory.
    Probe,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionAction {
    Request,
    OpenSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionObservation {
    pub permission: PermissionKey,
    /// PID of the process where this observation was made, not its OS grant owner.
    pub process_id: u32,
    pub state: PermissionState,
    pub evidence: PermissionEvidence,
    pub observed_at_ms: Option<i64>,
    pub error: Option<String>,
}

impl PermissionObservation {
    pub fn is_satisfied(&self) -> bool {
        self.process_id != 0
            && self.error.is_none()
            && self.evidence != PermissionEvidence::Unavailable
            && (self.evidence != PermissionEvidence::Probe || self.observed_at_ms.is_some())
            && matches!(
                self.state,
                PermissionState::Granted | PermissionState::NotRequired
            )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServicePermissions {
    /// Reporting service PID; individual observations may come from its sidecar.
    pub process_id: u32,
    pub permissions: Vec<PermissionObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPermission {
    pub component_id: String,
    pub component_name: String,
    pub permission: PermissionKey,
    pub feature: String,
    pub reason: String,
    /// None means that the actual executor could not be observed.
    pub observation: Option<PermissionObservation>,
    /// Collection failure, distinct from an observed OS denial.
    pub error: Option<String>,
    /// Mechanisms supported locally; remote callers can only read status.
    pub supported_actions: Vec<PermissionAction>,
}

impl HostPermission {
    pub fn is_satisfied(&self) -> bool {
        self.error.is_none()
            && self.observation.as_ref().is_some_and(|observation| {
                observation.permission == self.permission && observation.is_satisfied()
            })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionDeclarationError {
    pub component_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPermissions {
    pub host_id: String,
    pub host_version: String,
    pub platform: String,
    pub observed_at_ms: i64,
    pub permissions: Vec<HostPermission>,
    /// Missing, malformed or unsupported declarations must remain visible.
    /// An empty permission list is not proof that the inventory is complete.
    pub declaration_errors: Vec<PermissionDeclarationError>,
}

/// Local-control mutation. A Hub may later relay `GET /v1/permissions`; it must
/// not relay this request, and it does not store or decide the grant. The server
/// must verify the local control credential and Host identity. This DTO has no
/// caller-controlled isLocal flag or arbitrary Settings URL. The component
/// selects an installed service (or "host"); the key selects its permission.
/// The component is always explicit.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPermissionRequest {
    pub host_id: String,
    pub component_id: String,
    pub permission: PermissionKey,
}

pub const HOST_PERMISSIONS_PATH: &str = "/v1/permissions";
pub const HOST_PERMISSION_REQUEST_PATH: &str = "/internal/permissions/request";
pub const HOST_PERMISSION_SETTINGS_PATH: &str = "/internal/permissions/open-settings";
