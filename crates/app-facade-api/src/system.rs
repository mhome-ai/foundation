//! Space-owned runtime instances, Client sessions, and Hub-vantage Host inventory
//! and management. The Hub acts as the current Space member against LAN Hosts.
use core_api::host::management::HostRuntimeRequest;
use core_api::host::HostInfo;
use serde::{Deserialize, Serialize};
pub const INSTANCES_TARGET: &str = "/app/system/instances/get";
pub const CLIENTS_TARGET: &str = "/app/system/clients/get";
pub const HOSTS_TARGET: &str = "/app/system/hosts/get";
pub const HOSTS_RUNTIME_TARGET: &str = "/app/system/hosts/runtime";
pub const HOSTS_CLAIM_TARGET: &str = "/app/system/hosts/claim";
pub const INSTANCES_CONTRACT: &str = "mhome.system.instances.v1";
pub const CLIENTS_CONTRACT: &str = "mhome.system.clients.v1";
pub const HOSTS_CONTRACT: &str = "mhome.system.hosts.v1";
pub const HOSTS_RUNTIME_CONTRACT: &str = "mhome.system.hosts.runtime.v1";
pub const HOSTS_CLAIM_CONTRACT: &str = "mhome.system.hosts.claim.v1";
pub type HostsRuntimeRequest = HostRuntimeRequest<String>;
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstancesRequest {}
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
pub struct Instances {
    pub contract: String,
    pub scope_id: String,
    pub observed_at_ms: i64,
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
/// Hub-vantage machine observation. `source` is `local` for the Hub host or `mdns`
/// for other LAN records. `/info` is signed LAN for every reachable Host.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub host_id: String,
    pub host_name: String,
    pub host_type: String,
    pub source: String,
    pub reachable: bool,
    pub info: Observation<HostInfo>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hosts {
    pub contract: String,
    pub scope_id: String,
    pub observed_at_ms: i64,
    pub hosts: Vec<Host>,
}
/// Hub-vantage Host management. `result` is the native Host runtime payload
/// (`HostRuntimeResponse.data` or metrics).
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsRuntime {
    pub contract: String,
    pub scope_id: String,
    pub host_id: String,
    pub result: serde_json::Value,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostsClaimRequest {
    pub host_id: String,
}
/// Hub-vantage first-claim of an unused LAN Host for the current Space owner.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostsClaim {
    pub contract: String,
    pub scope_id: String,
    pub host_id: String,
    pub claimed: bool,
}
