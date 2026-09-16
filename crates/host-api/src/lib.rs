//! Machine facts only. No Space identities or runtime diagnostics belong here.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub service_id: String,
    pub version: String,
    pub kind: String,
    pub status: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInventory {
    pub host_id: String,
    pub observed_at_ms: i64,
    pub services: Vec<Service>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cpu {
    pub arch: String,
    pub logical_cores: u64,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capacity {
    pub total_bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpu {
    pub vendor: String,
    pub name: String,
    pub memory_bytes: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostInfo {
    pub host_id: String,
    pub host_name: String,
    pub host_type: String,
    pub os: String,
    pub os_version: Option<String>,
    pub cpu: Cpu,
    pub memory: Capacity,
    pub disk: Option<Capacity>,
    pub gpus: Vec<Gpu>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMetrics {
    pub service_id: String,
    pub status: String,
    pub sample_state: String,
    pub process_count: u64,
    pub cpu_usage_percent: Option<f64>,
    pub memory_resident_bytes: Option<u64>,
    pub disk_read_bytes_per_second: Option<f64>,
    pub disk_written_bytes_per_second: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostMetrics {
    pub host_id: String,
    pub sampled_at_ms: i64,
    pub cpu_usage_percent: Option<f64>,
    pub memory_available_bytes: Option<u64>,
    pub disk_available_bytes: Option<u64>,
    pub network_received_bytes_per_second: Option<f64>,
    pub network_sent_bytes_per_second: Option<f64>,
    pub services: Vec<ServiceMetrics>,
}
