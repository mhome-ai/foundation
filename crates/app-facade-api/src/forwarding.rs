//! Complete public calls carried only over an authenticated Cloud-to-Core Bridge.
//! This is distinct from internal domain dispatch and is never accepted as Client input.

use crate::FacadeCall;
use core_api::client_identity::{AuthenticatedClient, ClientPrincipal};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PREFIX: &str = "/invoke/app/";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ForwardedCaller {
    Client {
        identity: AuthenticatedClient,
    },
    Mcp {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "operationId")]
        operation_id: String,
    },
    Agent {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "operationId")]
        operation_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ForwardedCall {
    pub principal: ClientPrincipal,
    pub caller: ForwardedCaller,
    pub call: FacadeCall,
}

/// Preserve the public payload type across the Bridge's JSON response transport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub enum ForwardedResponse {
    Json(Value),
}

/// Trusted operation binding. MCP has no conversation thread or run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ActionInvocation {
    Agent {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "threadId")]
        thread_id: String,
        #[serde(rename = "runId")]
        run_id: String,
        #[serde(rename = "operationId")]
        operation_id: String,
    },
    Mcp {
        #[serde(rename = "surfaceId")]
        surface_id: String,
        #[serde(rename = "operationId")]
        operation_id: String,
    },
}

impl ActionInvocation {
    pub fn surface_id(&self) -> &str {
        match self {
            Self::Agent { surface_id, .. } | Self::Mcp { surface_id, .. } => surface_id,
        }
    }
    pub fn operation_id(&self) -> &str {
        match self {
            Self::Agent { operation_id, .. } | Self::Mcp { operation_id, .. } => operation_id,
        }
    }
    pub fn thread_id(&self) -> Option<&str> {
        match self {
            Self::Agent { thread_id, .. } => Some(thread_id),
            Self::Mcp { .. } => None,
        }
    }
    pub fn run_id(&self) -> Option<&str> {
        match self {
            Self::Agent { run_id, .. } => Some(run_id),
            Self::Mcp { .. } => None,
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        if [
            Some(self.surface_id()),
            Some(self.operation_id()),
            self.thread_id(),
            self.run_id(),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.trim().is_empty())
        {
            return Err("Invocation identifiers must not be empty");
        }
        Ok(())
    }
}
