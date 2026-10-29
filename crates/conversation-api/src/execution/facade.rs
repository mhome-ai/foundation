//! App Facade contract used by the canonical Agent implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution::{ActionId, ExternalError, InvocationContext, OperationId};

/// Canonical App Facade request selected by Agent-owned tool code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeRequest {
    /// Stable idempotency identifier for this tool operation.
    pub operation_id: OperationId,
    /// Canonical `/app/...` target. The Agent owns every tool-to-target mapping.
    pub target: String,
    /// Structured input validated and normalized by Agent-owned tool code.
    pub input: Value,
    /// Output projection, separate from the immutable business input.
    pub response_detail: crate::execution::ResponseDetail,
}

pub use crate::{InteractionCard, InteractionKind, InteractionTone};

/// Prepared operation awaiting an explicit commit or rejection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedAction {
    /// Stable action identifier used for idempotent commit and rejection.
    pub action_id: ActionId,
    /// Idempotency identifier supplied by Runtime.
    pub operation_id: OperationId,
    /// App Facade-generated preview suitable for user confirmation.
    pub card: InteractionCard,
    /// Opaque execution plan. Runtime persists it but never exposes it to the model or UI.
    #[serde(default)]
    pub payload: Value,
    /// App Facade revision that must still be current when committing.
    pub revision: u64,
}

/// Structured result returned by an App Facade operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeResult {
    /// In-process presentation provenance, assigned by the adapter from its request.
    /// Never inferred from the value type or sent on the wire. RPC adapters restore it.
    #[serde(skip)]
    pub response_detail: super::ResponseDetail,
    /// Canonical serialized `FacadeResponse`; never a bare business DTO.
    #[serde(deserialize_with = "deserialize_response")]
    pub value: Value,
    /// New prompt-context generation when this operation changed Agent-visible facts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_invalidation: Option<crate::execution::ContextInvalidation>,
}

/// Environment-specific implementation of the App Facade boundary.
///
/// Implementations may execute in-process (`MeowCore`) or through RPC (agent-cloud/Lion), but they
/// cannot supply prompts, tools, workflows, or model-facing schemas.
#[async_trait]
pub trait AppFacade: Send + Sync {
    /// Executes a direct App Facade request.
    async fn invoke(
        &self,
        context: &InvocationContext,
        request: FacadeRequest,
    ) -> Result<FacadeResult, ExternalError>;

    /// Prepares a mutating operation without applying it.
    async fn prepare(
        &self,
        context: &InvocationContext,
        request: FacadeRequest,
    ) -> Result<PreparedAction, ExternalError>;

    /// Commits a prepared operation idempotently.
    async fn commit(
        &self,
        context: &InvocationContext,
        action: PreparedAction,
    ) -> Result<FacadeResult, ExternalError>;

    /// Rejects a prepared operation idempotently.
    async fn reject(
        &self,
        context: &InvocationContext,
        action: PreparedAction,
    ) -> Result<(), ExternalError>;
}

fn deserialize_response<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    use serde::de::Error;
    let response = crate::execution::FacadeResponse::deserialize(deserializer)?;
    serde_json::to_value(response).map_err(D::Error::custom)
}
