//! OS authorization for one computer's Host and the services it runs; independent
//! of any Space.
//!
//! Each installed package declares requirements per OS. The same PermissionKey on
//! another process is a separate grant, because the operating system attributes
//! the call to the process that makes it. The snapshot therefore keeps one row
//! per declaring component, including stopped services, and does not merge those
//! rows. Merely reading this snapshot MUST NOT request OS authorization. Local
//! Network has no general passive status API: a probe result is historical
//! evidence, never a silently refreshed system setting.
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionState {
    Unknown,
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PermissionEvidence {
    /// A passive OS authorization query, not hardware availability.
    System,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPermission {
    pub permission: PermissionKey,
    pub state: PermissionState,
    pub evidence: PermissionEvidence,
    pub observed_at_ms: Option<i64>,
    pub uses: Vec<PermissionUse>,
    /// Mechanisms supported by this Host, NOT authority for a remote caller.
    pub supported_actions: Vec<PermissionAction>,
    pub error: Option<String>,
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
    pub subject_id: String,
    pub subject_name: String,
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
/// Older callers may omit the component only when ownership is unambiguous.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPermissionRequest {
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub component_id: Option<String>,
    pub permission: PermissionKey,
}

pub const HOST_PERMISSIONS_PATH: &str = "/v1/permissions";
pub const HOST_PERMISSION_REQUEST_PATH: &str = "/internal/permissions/request";
pub const HOST_PERMISSION_SETTINGS_PATH: &str = "/internal/permissions/open-settings";
