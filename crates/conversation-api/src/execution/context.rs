//! Invocation identity and tracing context.

use serde::{Deserialize, Serialize};

use crate::execution::{
    ClientId, ConversationSurface, OperationId, RunId, ScopeId, TenantId, ThreadId, UserId,
};

/// Ownership boundary used for authorization and persistence partitioning.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scope {
    /// Optional tenant for multi-tenant deployments.
    pub tenant_id: Option<TenantId>,
    /// Application scope that owns Agent state and application capabilities.
    pub scope_id: ScopeId,
}

/// Authenticated actor that initiated an Agent run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    /// Original authority; a Space Client never becomes a user through a relay.
    pub principal: crate::ClientPrincipal,
    /// Enrollment responsible for this run. Required for Space Clients.
    pub credential_id: Option<String>,
    /// Optional originating application client used for routed responses.
    pub client_id: Option<ClientId>,
}

impl Actor {
    #[must_use]
    pub fn user(user_id: impl Into<UserId>, client_id: Option<ClientId>) -> Self {
        Self {
            principal: crate::ClientPrincipal::User {
                user_id: user_id.into().as_str().to_owned(),
            },
            credential_id: None,
            client_id,
        }
    }

    pub fn user_id(&self) -> Option<&str> {
        self.principal.user_id()
    }

    /// Validate binding after transport authentication. This does not authorize tools.
    pub fn valid_for(&self, scope: &Scope, surface: &ConversationSurface) -> bool {
        if self
            .client_id
            .as_ref()
            .is_some_and(|id| !valid_id(id.as_str()))
            || self
                .credential_id
                .as_deref()
                .is_some_and(|id| !valid_id(id))
        {
            return false;
        }
        match &self.principal {
            crate::ClientPrincipal::User { user_id } => {
                valid_id(user_id)
                    && !surface.is_space_clients()
                    && surface.user_id().is_none_or(|owner| owner == user_id)
            }
            crate::ClientPrincipal::Space { .. } => {
                self.client_id.is_some()
                    && self.credential_id.is_some()
                    && self.principal.validate_for(
                        scope.tenant_id.as_ref().map_or("", |id| id.as_str()),
                        scope.scope_id.as_str(),
                    )
                    && surface.is_space_clients()
            }
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && !value.chars().any(char::is_control)
}

/// Context that follows one Agent invocation across all ports.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
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
