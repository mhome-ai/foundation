use serde::{Deserialize, Serialize};

pub const STATUS_TARGET: &str = "/status";
pub const STATUS_CHANGED_TARGET: &str = "/status/changed";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IdentityPhase {
    Unknown,
    Uncommissioned,
    Commissioning,
    Commissioned,
    Stale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum HubPhase {
    Unknown,
    WaitingDiscovery,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionPhase {
    Disconnected,
    Connecting,
    Authenticating,
    Authenticated,
    Connected,
    Retrying,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RuntimePhase {
    Starting,
    Ready,
    Retrying,
    Restarting,
    Failed,
    Corrupted,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProcessPhase {
    Unknown,
    Starting,
    Running,
    Restarting,
    Stopping,
    Failed,
}

/// Product-facing projection derived by the Node. Neither the Hub nor a
/// supervisor may use this value to drive lifecycle transitions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EffectivePhase {
    ProcessStarting,
    Uncommissioned,
    Commissioning,
    WaitingForHub,
    Connecting,
    Authenticating,
    Online,
    PartiallyOnline,
    Degraded,
    Retrying,
    Failed,
    Stopping,
}

/// Status of one commissioned identity shell. Runtime fields are a read-only
/// projection of the separately reported logical Runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceSnapshot {
    pub node_id: String,
    pub runtime_id: String,
    pub hub_id: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub identity: IdentityPhase,
    pub hub: HubPhase,
    pub connection: ConnectionPhase,
    pub runtime: RuntimePhase,
    pub runtime_generation: u64,
    pub connection_generation: u64,
    pub revision: u64,
    pub effective: EffectivePhase,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_retry_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_retry_after_ms: Option<u64>,
    pub updated_at_ms: i64,
}

/// Authoritative state of one logical business Runtime. Several identity
/// shells may reference this object for singleton/shared Node policies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeSnapshot {
    pub runtime_id: String,
    pub runtime: RuntimePhase,
    pub generation: u64,
    pub revision: u64,
    #[serde(default)]
    pub member_node_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after_ms: Option<u64>,
    pub updated_at_ms: i64,
}

/// Complete status reported by one authenticated Node identity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub contract: String,
    pub service_id: String,
    pub node_type: String,
    pub process_generation: String,
    pub revision: u64,
    pub process: ProcessPhase,
    pub instance: InstanceSnapshot,
    pub runtime: RuntimeSnapshot,
    pub updated_at_ms: i64,
}

/// Hub-normalized payload stored inside a public NodeInstance resource.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InstanceStatus {
    pub contract: String,
    pub service_id: String,
    pub node_type: String,
    pub process_generation: String,
    pub revision: u64,
    pub process: ProcessPhase,
    pub instance: InstanceSnapshot,
    pub updated_at_ms: i64,
}

/// Hub-normalized payload stored inside a public NodeRuntime resource.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatus {
    pub contract: String,
    pub service_id: String,
    pub node_type: String,
    pub process_generation: String,
    pub revision: u64,
    pub runtime: RuntimeSnapshot,
    pub updated_at_ms: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_wire_shape_is_camel_case_and_additive() {
        let value = serde_json::json!({
            "contract": crate::node::STATUS_CONTRACT,
            "serviceId": "camera",
            "nodeType": "camera",
            "processGeneration": "process-1",
            "revision": 2,
            "process": "running",
            "instance": {
                "nodeId": "node-1",
                "runtimeId": "instance:node-1",
                "hubId": "hub-1",
                "tenantId": "tenant-1",
                "scopeId": "scope-1",
                "identity": "commissioned",
                "hub": "resolved",
                "connection": "connected",
                "runtime": "ready",
                "runtimeGeneration": 3,
                "connectionGeneration": 4,
                "revision": 5,
                "effective": "online",
                "updatedAtMs": 6,
                "future": true
            },
            "runtime": {
                "runtimeId": "instance:node-1",
                "runtime": "ready",
                "generation": 3,
                "revision": 7,
                "memberNodeIds": ["node-1"],
                "updatedAtMs": 8
            },
            "updatedAtMs": 9,
            "future": true
        });
        let status: StatusPayload = serde_json::from_value(value).unwrap();
        assert_eq!(status.instance.node_id, "node-1");
        assert_eq!(status.runtime.runtime_id, "instance:node-1");
    }
}
