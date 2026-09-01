use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::NodeInstancePolicy;

pub const NODE_DESCRIBE_SCHEMA_VERSION: &str = "mhome.node.describe.v1";
pub const NODE_READINESS_SCHEMA_VERSION: &str = "mhome.node.readiness.v1";
pub const NODE_SERVICE_PROTOCOL_V1_SCHEMA: &str =
    include_str!("../contract/node-service-protocol-v1.json");

pub fn node_describe_target(node_type: &str) -> String {
    format!("/{node_type}/describe")
}

pub fn node_readiness_target(node_type: &str) -> String {
    format!("/{node_type}/readiness")
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDescribeRequest {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeDescription {
    pub schema_version: String,
    pub node_type: String,
    pub service_id: String,
    pub service_version: String,
    pub instance_policy: NodeInstancePolicy,
    pub routes: Vec<String>,
    pub capabilities: Vec<String>,
    pub details: Map<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeReadinessRequest {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeReadinessState {
    Starting,
    Ready,
    Degraded,
    Failed,
    Stopping,
    Stopped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeReadinessReason {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeReadinessSnapshot {
    pub schema_version: String,
    pub node_type: String,
    pub service_id: String,
    pub service_version: String,
    pub process_generation: String,
    pub state: NodeReadinessState,
    pub ready: bool,
    pub revision: u64,
    pub updated_at_ms: i64,
    pub reason: Option<NodeReadinessReason>,
    pub details: Map<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn empty_requests_reject_unknown_fields() {
        assert!(serde_json::from_value::<NodeDescribeRequest>(json!({})).is_ok());
        assert!(serde_json::from_value::<NodeReadinessRequest>(json!({})).is_ok());
        assert!(
            serde_json::from_value::<NodeDescribeRequest>(json!({"nodeId": "legacy"})).is_err()
        );
        assert!(serde_json::from_value::<NodeReadinessRequest>(json!({"refresh": true})).is_err());
    }

    #[test]
    fn response_contracts_are_additive_for_consumers() {
        let description: NodeDescription = serde_json::from_value(json!({
            "schemaVersion": NODE_DESCRIBE_SCHEMA_VERSION,
            "nodeType": "camera",
            "serviceId": "camera",
            "serviceVersion": "1.0.0",
            "instancePolicy": "multiple",
            "routes": ["/camera/describe", "/camera/readiness"],
            "capabilities": [],
            "details": {},
            "futureField": true
        }))
        .expect("additive description");
        assert_eq!(description.node_type, "camera");

        let readiness: NodeReadinessSnapshot = serde_json::from_value(json!({
            "schemaVersion": NODE_READINESS_SCHEMA_VERSION,
            "nodeType": "camera",
            "serviceId": "camera",
            "serviceVersion": "1.0.0",
            "processGeneration": "generation-1",
            "state": "degraded",
            "ready": true,
            "revision": 2,
            "updatedAtMs": 3,
            "reason": null,
            "details": {},
            "futureField": true
        }))
        .expect("additive readiness");
        assert_eq!(readiness.state, NodeReadinessState::Degraded);
    }

    #[test]
    fn canonical_schema_declares_every_wire_field_and_policy() {
        let schema: Value = serde_json::from_str(NODE_SERVICE_PROTOCOL_V1_SCHEMA).unwrap();
        assert_eq!(
            schema["$defs"]["emptyRequest"]["additionalProperties"],
            false
        );
        assert_eq!(schema["$defs"]["describe"]["additionalProperties"], true);
        assert_eq!(
            schema["$defs"]["readinessReason"]["additionalProperties"],
            true
        );
        assert_eq!(schema["$defs"]["readiness"]["additionalProperties"], true);
        assert_eq!(
            schema["$defs"]["describe"]["properties"]["schemaVersion"]["const"],
            NODE_DESCRIBE_SCHEMA_VERSION
        );
        assert_eq!(
            schema["$defs"]["readiness"]["properties"]["schemaVersion"]["const"],
            NODE_READINESS_SCHEMA_VERSION
        );
        assert_eq!(
            schema["$defs"]["describe"]["properties"]["instancePolicy"]["enum"],
            json!(["multiple", "singleton", "shared"])
        );
    }
}
