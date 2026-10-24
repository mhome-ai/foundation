//! Host ingress and cloud-authoritative relationship replication. These are internal protocols.
use super::{ExternalActor, MessagingAddress, MessagingModelError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RELATIONSHIP_RESOURCE: &str = "messaging.relationships";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingCloudRequest {
    pub tenant_id: String,
    pub scope_id: String,
    pub operation: MessagingIngressOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", content = "input", rename_all = "snake_case")]
pub enum MessagingIngressOperation {
    Register(IngressRegister),
    Redeem(IngressRedeem),
    Unregister(IngressUnregister),
    GrantCreate(IngressGrant),
    GrantDelete(IngressGrant),
    BindingUpdate(IngressBindingUpdate),
    Unbind(IngressUnbind),
}
impl MessagingIngressOperation {
    pub fn target(&self) -> &'static str {
        match self {
            Self::Register(_) => "/messaging/ingress/register",
            Self::Redeem(_) => "/messaging/ingress/redeem",
            Self::Unregister(_) => "/messaging/ingress/unregister",
            Self::GrantCreate(_) => "/messaging/ingress/grant/create",
            Self::GrantDelete(_) => "/messaging/ingress/grant/delete",
            Self::BindingUpdate(_) => "/messaging/ingress/binding/update",
            Self::Unbind(_) => "/messaging/ingress/unbind",
        }
    }
    pub fn payload(&self) -> Result<String, serde_json::Error> {
        match self {
            Self::Register(input) => serde_json::to_string(input),
            Self::Redeem(input) => serde_json::to_string(input),
            Self::Unregister(input) => serde_json::to_string(input),
            Self::GrantCreate(input) | Self::GrantDelete(input) => serde_json::to_string(input),
            Self::BindingUpdate(input) => serde_json::to_string(input),
            Self::Unbind(input) => serde_json::to_string(input),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressRegister {
    pub provider: String,
    pub account_id: String,
    pub instance_id: String,
    pub owner_user_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressRegistered {
    pub resource_id: String,
    pub instance_id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityCodeKind {
    Link,
    Bind,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressRedeem {
    pub resource_id: String,
    pub code: String,
    pub kind: IdentityCodeKind,
    pub context: IdentityRedemptionContext,
}

/// Receiving identity facts for a parsed command. Message content and
/// its schema belong to the receiving adapter, not to identity redemption.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdentityRedemptionContext {
    pub event_id: String,
    pub address: MessagingAddress,
    pub actor: ExternalActor,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_display_name: Option<String>,
}

impl IdentityRedemptionContext {
    pub fn validate(self) -> Result<Self, MessagingModelError> {
        let event_id = super::model::segment(self.event_id, "messaging event id is invalid")?;
        let address = self.address.validate()?.base_address();
        let actor = self.actor.validate()?;
        if actor.provider != address.provider || actor.account_id != address.account_id {
            return Err(MessagingModelError(
                "messaging actor and address account do not match",
            ));
        }
        super::model::optional_ref(
            &self.conversation_display_name,
            "messaging conversation display name is invalid",
        )?;
        Ok(Self {
            event_id,
            address,
            actor,
            conversation_display_name: self.conversation_display_name,
        })
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressRedemption {
    pub challenge_id: String,
    pub state: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressUnregister {
    pub resource_id: String,
    pub owner_user_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingRelationshipApply {
    pub tenant_id: String,
    pub scope_id: String,
    pub resource_id: String,
    pub snapshot: MessagingRelationshipsSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingRelationshipsSnapshot {
    pub provider: String,
    pub account_id: String,
    pub instance_id: String,
    pub revoked: bool,
    pub links: Vec<MessagingLinkedIdentity>,
    pub bindings: Vec<MessagingGroupBinding>,
    pub grants: Vec<MessagingSpaceGrant>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingLinkedIdentity {
    pub link_id: String,
    pub external_user_id: String,
    pub tenant_id: String,
    pub user_id: String,
    pub display_name: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingGroupBinding {
    pub binding_id: String,
    pub conversation_id: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub bound_by_user_id: String,
    pub display_name: Option<String>,
    pub created_at_ms: i64,
    pub projections: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingSpaceGrant {
    pub tenant_id: String,
    pub scope_id: String,
    pub granted_by_user_id: String,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressGrant {
    pub resource_id: String,
    pub scope_id: String,
    pub user_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressBindingUpdate {
    pub resource_id: String,
    pub scope_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub projections: Value,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IngressUnbind {
    pub resource_id: String,
    pub scope_id: String,
    pub user_id: String,
    pub conversation_id: String,
    pub binding_id: String,
}
