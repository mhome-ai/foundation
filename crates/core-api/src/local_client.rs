//! Lion-issued local credentials. Signature, issuer, audience and lifetime must
//! be checked by the Core before these claims become an authenticated identity.
use crate::client_identity::{AuthenticatedClient, ClientIdentityError, ClientPrincipal};
use serde::{Deserialize, Serialize};

pub const LOCAL_CLIENT_AUDIENCE: &str = "mhome:local-client";
pub const USER_TOKEN_TYPE: &str = "local_client_access";
pub const SPACE_TOKEN_TYPE: &str = "local_space_client_access";
pub const REGISTER_SPACE_CLIENT_TARGET: &str = "/hub/client/space/register";

#[derive(Clone, Serialize, Deserialize)]
pub struct LocalClientClaims {
    pub sub: String,
    pub token_type: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub hub_id: String,
    pub jti: String,
    pub exp: usize,
    pub iat: usize,
    #[serde(default)]
    pub is_test: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_client_id: Option<String>,
}

impl LocalClientClaims {
    /// The registered installation comes from Core's credential store, never
    /// from an ordinary request's clientInfo or identity-shaped payload fields.
    pub fn client(&self, registered_id: &str) -> Result<AuthenticatedClient, ClientIdentityError> {
        let principal = match self.token_type.as_str() {
            USER_TOKEN_TYPE if self.app_client_id.is_none() => ClientPrincipal::User {
                user_id: self.sub.clone(),
            },
            SPACE_TOKEN_TYPE
                if self.sub == self.scope_id
                    && self.app_client_id.as_deref() == Some(registered_id) =>
            {
                ClientPrincipal::Space {
                    tenant_id: self.tenant_id.clone(),
                    scope_id: self.scope_id.clone(),
                }
            }
            _ => return Err(ClientIdentityError::InvalidIdentifier),
        };
        let client = AuthenticatedClient {
            app_client_id: registered_id.to_owned(),
            principal,
            credential_id: Some(self.jti.clone()),
        };
        client.validate_for(&self.tenant_id, &self.scope_id)?;
        Ok(client)
    }
}

/// Accepted only on the authenticated Lion-to-Core bridge. The signed token
/// binds the Space, Hub, installation and enrollment; the other fields are labels.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegisterSpaceClientRequest {
    pub jwt_token: String,
    pub client_source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpaceClientCredentialStatus {
    pub credential_id: String,
    pub status: String,
    pub activate_before: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(kind: &str) -> LocalClientClaims {
        LocalClientClaims {
            sub: "scope".into(),
            token_type: kind.into(),
            tenant_id: "tenant".into(),
            scope_id: "scope".into(),
            hub_id: "hub".into(),
            jti: "enrollment".into(),
            exp: 100,
            iat: 1,
            is_test: false,
            app_client_id: Some("a:device".into()),
        }
    }

    #[test]
    fn space_credential_has_no_user_and_binds_its_installation() {
        let claims = claims(SPACE_TOKEN_TYPE);
        let client = claims.client("a:device").unwrap();
        assert_eq!(client.user_id(), None);
        assert!(client.is_space_client());
        assert!(claims.client("a:another").is_err());
        assert!(client.validate_for("tenant", "another").is_err());
    }

    #[test]
    fn user_and_space_claims_cannot_be_mixed() {
        let mut claims = claims(USER_TOKEN_TYPE);
        assert!(claims.client("a:device").is_err());
        claims.app_client_id = None;
        claims.sub = "user".into();
        assert_eq!(claims.client("a:device").unwrap().user_id(), Some("user"));
        claims.token_type = SPACE_TOKEN_TYPE.into();
        assert!(claims.client("a:device").is_err());
        claims.app_client_id = Some("a:device".into());
        assert!(claims.client("a:device").is_err());
    }
}
