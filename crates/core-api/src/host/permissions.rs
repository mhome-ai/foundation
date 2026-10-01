//! OS authorization for one computer's Host and the services it runs; independent
//! of any Space.
//!
//! Services declare requirements and report observations from the processes that
//! actually access the resource. An executing process is not necessarily a
//! separate OS authorization identity: helpers can share a responsible app.
//! Host groups only established authorization subjects and preserves every
//! service's observations. Reading a report must never request authorization.
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionUse {
    pub component_id: String,
    pub component_name: String,
    pub feature: String,
    pub reason: String,
    /// Measured in the actual executor, including a sidecar when applicable.
    /// None means that the executor could not be observed; never copy a sibling's grant.
    pub observation: Option<PermissionObservation>,
    /// Collection/declaration failure, separate from a successfully observed OS denial.
    pub error: Option<String>,
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

/// A subject established for this permission and launch environment. Neither a
/// common parent nor equal permission states are enough to establish a subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionSubject {
    pub id: String,
    pub name: String,
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
    pub permission: PermissionKey,
    /// None keeps this component separate until its authorization scope is known.
    pub subject: Option<PermissionSubject>,
    pub uses: Vec<PermissionUse>,
    /// A currently available executor chosen by Host. No executor means no request.
    pub request_component_id: Option<String>,
    /// Mechanisms supported by this Host, NOT authority for a remote caller.
    pub supported_actions: Vec<PermissionAction>,
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
/// The component is always explicit, including when Host has grouped a shared grant.
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
