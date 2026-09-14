//! LAN Node commissioning. Cancellation is deliberately outside this contract.
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

pub const NODE_PREFLIGHT_SCHEMA_VERSION: &str = "mhome.node.preflight.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningAcceptRequest {
    pub token: String,
    pub node_id: String,
    pub hub_id: String,
    pub hub_url: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningAcceptResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningErrorResponse {
    pub ok: bool,
    pub reason: String,
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningPrepareRequest {
    #[serde(flatten)]
    pub commissioning: NodeCommissioningAcceptRequest,
    pub idempotency_key: String,
    pub expected_revision: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningStatusRequest {
    #[serde(flatten)]
    pub commissioning: NodeCommissioningAcceptRequest,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodePreflightState {
    Preparing,
    Ready,
    Failed,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePreflightError {
    pub code: String,
    pub detail: String,
    pub retryable: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodePreflightReadiness {
    pub schema_version: String,
    pub state: NodePreflightState,
    pub revision: u64,
    pub commissionable: bool,
    pub runtime_usable: bool,
    pub error: Option<NodePreflightError>,
    pub details: Map<String, Value>,
}
impl NodePreflightReadiness {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != NODE_PREFLIGHT_SCHEMA_VERSION {
            return Err("unsupported preflight schema");
        }
        if self.commissionable && (!self.runtime_usable || self.state != NodePreflightState::Ready)
        {
            return Err("commissionable requires a ready, usable runtime");
        }
        if (self.state == NodePreflightState::Failed) != self.error.is_some() {
            return Err("failed preflight requires an error, other states must not contain one");
        }
        if self
            .error
            .as_ref()
            .is_some_and(|e| e.code.trim().is_empty() || e.detail.trim().is_empty())
        {
            return Err("preflight error must include code and detail");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningPrepareResponse {
    pub ok: bool,
    pub started: bool,
    pub readiness: NodePreflightReadiness,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCommissioningStatusResponse {
    pub ok: bool,
    pub readiness: NodePreflightReadiness,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_private_readiness_and_inconsistent_success() {
        assert!(serde_json::from_value::<NodePreflightReadiness>(
            serde_json::json!({"commissionable":true})
        )
        .is_err());
        let mut r = NodePreflightReadiness {
            schema_version: NODE_PREFLIGHT_SCHEMA_VERSION.into(),
            state: NodePreflightState::Ready,
            revision: 1,
            commissionable: true,
            runtime_usable: true,
            error: None,
            details: Map::new(),
        };
        assert!(r.validate().is_ok());
        r.runtime_usable = false;
        assert!(r.validate().is_err());
        r.runtime_usable = true;
        r.schema_version = "matter.readiness.v1".into();
        assert!(r.validate().is_err());
    }
}
