//! App Facade contract used by the canonical Agent implementation.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution::{
    ActionId, ApprovalRequirement, ExternalError, InvocationContext, OperationId,
};

/// Canonical App Facade request selected by Agent-owned tool code.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FacadeRequest {
    /// Stable idempotency identifier for this tool operation.
    pub operation_id: OperationId,
    /// Canonical `/app/...` target. The Agent owns every tool-to-target mapping.
    pub target: String,
    /// Structured input validated and normalized by Agent-owned tool code.
    pub input: Value,
}

pub use crate::{
    ClientTask, InteractionKind, InteractionPreview, InteractionTone, PreviewDetail, PreviewValue,
};

/// Prepared operation awaiting an explicit commit or rejection.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PreparedAction {
    /// Stable action identifier used for idempotent commit and rejection.
    pub action_id: ActionId,
    /// Idempotency identifier supplied by Runtime.
    pub operation_id: OperationId,
    /// App Facade-generated preview suitable for user confirmation.
    pub preview: InteractionPreview,
    /// Opaque execution plan. Runtime persists it but never exposes it to the model or UI.
    #[serde(default)]
    pub payload: Value,
    /// App Facade revision that must still be current when committing.
    pub revision: u64,
    /// Approval requirement computed from the canonical prepared operation.
    pub approval: ApprovalRequirement,
    /// Presentation/continuation kind for clients.
    pub interaction_kind: InteractionKind,
}

/// Structured result returned by an App Facade operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FacadeResult {
    /// App Facade result payload.
    pub value: Value,
    /// A new prepared continuation when committing a user action still requires user work.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<PreparedAction>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_preview_rejects_incomplete_client_work() {
        let mut preview = InteractionPreview::confirmation(
            "integration.install",
            "Integration required",
            "Install it, then continue.",
        );
        preview.client_task = Some(ClientTask::InstallIntegrations {
            integration_ids: Vec::new(),
        });
        assert!(!preview.is_valid());

        preview.client_task = Some(ClientTask::InstallIntegrations {
            integration_ids: vec!["matter".to_owned()],
        });
        assert!(preview.is_valid());
        let encoded = serde_json::to_value(preview).expect("serializes");
        assert_eq!(encoded["clientTask"]["type"], "install_integrations");
        assert_eq!(encoded["clientTask"]["integrationIds"][0], "matter");
    }
}
