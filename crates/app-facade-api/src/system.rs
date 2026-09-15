//! Hub-vantage LAN inventory. Space instances come only from commissioned Core state.
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub const INVENTORY_TARGET: &str = "/app/system/inventory/get";
pub const CLIENTS_TARGET: &str = "/app/system/clients/get";
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Service {
    pub service_id: String,
    pub version: String,
    pub kind: String,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceInventory {
    pub host_id: String,
    pub observed_at_ms: i64,
    pub services: Vec<Service>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Host {
    pub host_id: String,
    pub host_name: String,
    pub host_type: String,
    pub is_hub_host: bool,
    pub discovered: bool,
    pub reachable: bool,
    pub observed_at_ms: i64,
    pub info: Option<Value>,
    pub info_error: Option<String>,
    pub services: Vec<Service>,
    pub services_error: Option<String>,
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
    pub connected: bool,
    pub health: String,
    pub health_observed_at_ms: Option<i64>,
    pub cloud_connected: Option<bool>,
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
