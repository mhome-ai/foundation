//! Authenticated application identity, independent of transport and device form.
//!
//! These types carry the result of credential verification. Deserializing them
//! never authenticates a caller. Only trusted ingress may attach them to a
//! request; ordinary request bodies and MWS clientInfo cannot select identity.

use serde::{Deserialize, Serialize};

/// The authority under which a Client acts. A Space principal is a restricted
/// Client enrolled in one Space, not an owner or the Space's Core service.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

/// `appClientId` identifies the installation, `credentialId` its current
/// enrollment. Neither is a secret. A connection ID is a separate routing key.
/// Personal Pods have a User principal and an enrollment credential, just as
/// Space Pods do; a normal user login need not have an enrollment credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthenticatedClient {
    pub app_client_id: String,
    pub principal: ClientPrincipal,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientIdentityError {
    InvalidIdentifier,
    MissingEnrollment,
    SpaceMismatch,
}

impl std::fmt::Display for ClientIdentityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::InvalidIdentifier => "Invalid authenticated Client identifier",
            Self::MissingEnrollment => "Space Client enrollment is required",
            Self::SpaceMismatch => "Client belongs to a different Space",
        })
    }
}

impl std::error::Error for ClientIdentityError {}

impl AuthenticatedClient {
    /// Structural and target binding validation, after credential verification.
    /// A User principal still requires current membership/role authorization at
    /// the operation boundary. This check never grants access by itself.
    pub fn validate_for(&self, tenant_id: &str, scope_id: &str) -> Result<(), ClientIdentityError> {
        let subject = self.app_client_id.strip_prefix("a:");
        if !valid_id(tenant_id)
            || !valid_id(scope_id)
            || !subject.is_some_and(|value| {
                valid_id(value) && !value.starts_with("a:") && !value.starts_with("p:")
            })
            || self
                .credential_id
                .as_deref()
                .is_some_and(|id| !valid_id(id))
        {
            return Err(ClientIdentityError::InvalidIdentifier);
        }
        match &self.principal {
            ClientPrincipal::User { user_id } if !valid_id(user_id) => {
                Err(ClientIdentityError::InvalidIdentifier)
            }
            ClientPrincipal::User { .. } => Ok(()),
            ClientPrincipal::Space {
                tenant_id: bound_tenant,
                scope_id: bound_scope,
            } => {
                if self.credential_id.is_none() {
                    return Err(ClientIdentityError::MissingEnrollment);
                }
                if bound_tenant != tenant_id || bound_scope != scope_id {
                    return Err(ClientIdentityError::SpaceMismatch);
                }
                Ok(())
            }
        }
    }

    /// No initializer/owner fallback: a Space Client has no acting user.
    pub fn user_id(&self) -> Option<&str> {
        match &self.principal {
            ClientPrincipal::User { user_id } => Some(user_id),
            ClientPrincipal::Space { .. } => None,
        }
    }

    pub fn is_space_client(&self) -> bool {
        matches!(self.principal, ClientPrincipal::Space { .. })
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
