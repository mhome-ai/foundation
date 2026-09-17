//! Native Client to Host wire protocol. No Space identity.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRuntimeRequest<C> {
    pub host_id: String,
    #[serde(flatten)]
    pub action: HostRuntimeAction<C>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum HostRuntimeAction<C> {
    Inspect,
    Metrics,
    RestartOperation {
        component: C,
        #[serde(rename = "operationId")]
        operation_id: String,
    },
    Plan {
        components: Vec<C>,
        #[serde(default)]
        all: bool,
    },
    Start {
        #[serde(rename = "planId")]
        plan_id: String,
    },
    Operation {
        #[serde(rename = "operationId")]
        operation_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRuntimeResponse {
    pub host_id: String,
    pub data: serde_json::Value,
}
