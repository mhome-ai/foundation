//! Authenticated caller authority shared by Client ingress and Agent execution.
//! Decoding a principal does not authenticate it; only trusted ingress supplies it.

use crate::{ConversationSurface, SurfaceParseError};
use serde::{Deserialize, Serialize};

/// The authority under which a Client acts. A Space principal is a restricted
/// Client enrolled in one Space, not an owner or the Space's Core service.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ClientPrincipal {
    User {
        #[serde(rename = "userId")]
        user_id: String,
    },
    Space {
        #[serde(rename = "tenantId")]
        tenant_id: String,
        #[serde(rename = "scopeId")]
        scope_id: String,
    },
}

impl ClientPrincipal {
    /// No initializer, billing owner or service identity fallback.
    pub fn user_id(&self) -> Option<&str> {
        match self {
            Self::User { user_id } => Some(user_id),
            Self::Space { .. } => None,
        }
    }

    pub fn validate_for(&self, tenant_id: &str, scope_id: &str) -> bool {
        if !valid_id(tenant_id) || !valid_id(scope_id) {
            return false;
        }
        match self {
            Self::User { user_id } => valid_id(user_id),
            Self::Space {
                tenant_id: tenant,
                scope_id: scope,
            } => tenant == tenant_id && scope == scope_id,
        }
    }

    /// Conversation selection is owned by authenticated ingress, never request payloads.
    /// The caller must retain tenant and scope alongside the returned surface.
    pub fn client_surface(
        &self,
        tenant_id: &str,
        scope_id: &str,
    ) -> Result<ConversationSurface, SurfaceParseError> {
        if !self.validate_for(tenant_id, scope_id) {
            return Err(SurfaceParseError);
        }
        match self {
            Self::User { user_id } => ConversationSurface::client_personal(user_id),
            Self::Space { .. } => Ok(ConversationSurface::space_clients()),
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
