//! Hub-vantage LAN inventory. Space instances come only from commissioned Core state.
pub use host_api::permissions::HostPermissions;
pub use host_api::{HostInfo, HostMetrics, Service};
use serde::{Deserialize, Serialize};
pub const INVENTORY_TARGET: &str = "/app/system/inventory/get";
pub const CLIENTS_TARGET: &str = "/app/system/clients/get";
pub const PERMISSIONS_TARGET: &str = "/app/system/host/permissions/get";
pub const METRICS_TARGET: &str = "/app/system/host/metrics/get";
pub const INVENTORY_CONTRACT: &str = "mhome.system.inventory.v1";
pub const CLIENTS_CONTRACT: &str = "mhome.system.clients.v1";
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryRequest {}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MetricsRequest {
    pub host_id: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PermissionsRequest {
    pub host_id: String,
}
/// Read-only Hub-vantage projection. Local management uses the native Client
/// route and never requires a Space. OS actions are intentionally not exposed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Permissions {
    pub host_id: String,
    pub snapshot: Observation<HostPermissions>,
}
/// Query success and data freshness are independent; failed refreshes retain the last observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation<T> {
    pub status: ObservationStatus,
    pub data: Option<T>,
    pub observed_at_ms: Option<i64>,
    pub last_attempt_at_ms: Option<i64>,
    pub stale: bool,
    pub error_code: Option<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObservationStatus {
    Ok,
    Unavailable,
    Unsupported,
    NotQueried,
}
impl<T> Default for Observation<T> {
    fn default() -> Self {
        Self {
            status: ObservationStatus::NotQueried,
            data: None,
            observed_at_ms: None,
            last_attempt_at_ms: None,
            stale: false,
            error_code: None,
        }
    }
}
impl<T> Observation<T> {
    pub fn success(&mut self, data: T, observed: i64, attempted: i64) {
        self.status = ObservationStatus::Ok;
        self.data = Some(data);
        self.observed_at_ms = Some(observed);
        self.last_attempt_at_ms = Some(attempted);
        self.stale = false;
        self.error_code = None;
    }
    pub fn failed(&mut self, status: ObservationStatus, code: &str, attempted: i64) {
        self.status = status;
        self.last_attempt_at_ms = Some(attempted);
        self.stale = self.data.is_some();
        self.error_code = Some(code.into());
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    pub host_id: String,
    pub sample: Observation<HostMetrics>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub host_id: String,
    pub host_name: String,
    pub host_type: String,
    pub is_hub_host: bool,
    pub source: String,
    pub reachable: bool,
    pub info: Observation<HostInfo>,
    pub services: Observation<Vec<Service>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Instance {
    pub id: String,
    pub kind: String,
    pub host_id: String,
    pub service_id: String,
    pub display_name: String,
    pub node_type: Option<String>,
    pub hub_id: String,
    pub commissioned: bool,
    pub lifecycle: Option<String>,
    pub hub_connection: Option<String>,
    pub cloud_connection: Option<String>,
    pub health: Observation<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    pub contract: String,
    pub scope_id: String,
    pub observed_at_ms: i64,
    pub discovery_error: Option<String>,
    pub hosts: Vec<Host>,
    pub instances: Vec<Instance>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientSession {
    pub app_client_id: String,
    pub session_id: String,
    pub source: String,
    pub device_type: String,
    pub is_current: bool,
    pub route_kind: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clients {
    pub contract: String,
    pub scope_id: String,
    pub observed_at_ms: i64,
    pub sessions: Vec<ClientSession>,
}

/// Concrete facade inputs. No caller-supplied URL, path or arbitrary Host action.
pub const INSPECT_TARGET: &str = "/app/system/host/inspect";
pub const PLAN_TARGET: &str = "/app/system/host/plan";
pub const START_TARGET: &str = "/app/system/host/start";
pub const OPERATION_TARGET: &str = "/app/system/host/operation/get";
pub const RESTART_TARGET: &str = "/app/system/host/restart";
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlanRequest {
    pub host_id: String,
    pub components: Vec<String>,
    #[serde(default)]
    pub all: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRequest {
    pub host_id: String,
    pub plan_id: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationRequest {
    pub host_id: String,
    pub operation_id: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RestartRequest {
    pub host_id: String,
    pub component: String,
    pub operation_id: String,
}
