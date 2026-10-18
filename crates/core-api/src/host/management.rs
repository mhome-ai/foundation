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
    AdministratorRequest {
        proof: String,
    },
    AdministratorRequestStatus {
        proof: String,
    },
    AdministratorRequestCancel {
        proof: String,
    },
    Administrators,
    AdministratorDecide {
        #[serde(rename = "requestId")]
        request_id: String,
        approve: bool,
    },
    AdministratorRemove {
        #[serde(rename = "userId")]
        user_id: String,
    },
    ConfigureService {
        #[serde(rename = "serviceId")]
        service_id: String,
        enabled: bool,
    },
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

impl<C> HostRuntimeAction<C> {
    /// Applicant credentials are accepted only by this separate, bounded route.
    pub fn applicant_proof(&self) -> Option<&str> {
        match self {
            Self::AdministratorRequest { proof }
            | Self::AdministratorRequestStatus { proof }
            | Self::AdministratorRequestCancel { proof } => Some(proof),
            _ => None,
        }
    }
    pub fn path(&self) -> &'static str {
        if self.applicant_proof().is_some() {
            "/v1/auth/administrators"
        } else if matches!(self, Self::Metrics) {
            "/metrics"
        } else {
            "/v1/runtime"
        }
    }
}
