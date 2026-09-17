//! Invocation identity and tracing context.

use serde::{Deserialize, Serialize};

use crate::execution::{
    ClientId, ConversationSurface, OperationId, RunId, ScopeId, TenantId, ThreadId, UserId,
};

/// Ownership boundary used for authorization and persistence partitioning.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct Scope {
    /// Optional tenant for multi-tenant deployments.
    pub tenant_id: Option<TenantId>,
    /// Application scope that owns Agent state and application capabilities.
    pub scope_id: ScopeId,
}

/// Authenticated actor that initiated an Agent run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    /// Authenticated user identity.
    pub user_id: UserId,
    /// Optional originating application client used for routed responses.
    pub client_id: Option<ClientId>,
}

/// Context that follows one Agent invocation across all ports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct InvocationContext {
    /// Authorization and persistence scope.
    pub scope: Scope,
    /// Authenticated actor; never accepted from model-generated arguments.
    pub actor: Actor,
    /// Trusted, canonical conversation space. Runtime uses it only as thread identity.
    pub surface_id: ConversationSurface,
    /// Durable conversation thread.
    pub thread_id: ThreadId,
    /// Current run identifier.
    pub run_id: RunId,
    /// Idempotency identifier for the initiating operation.
    pub operation_id: OperationId,
    /// Optional absolute Unix deadline in milliseconds.
    pub deadline_unix_ms: Option<u64>,
    /// Optional W3C traceparent propagated by the deployment.
    pub traceparent: Option<String>,
}
