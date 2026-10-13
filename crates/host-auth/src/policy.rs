//! Host management policy, independent of token transport and persistence.
//!
//! A verified grant may restrict these rules, never widen them. In particular a
//! Space Client remains read-only even if an issuer accidentally grants writes.
//! Adapters authenticate controller kinds; request JSON cannot select a kind.

use crate::token_session::{Binding, ControllerKind, Principal};
use core_api::RuntimeEnv;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HostMode {
    Admin,
    Space,
}

/// Personal Pods are User Clients. Shared Pods are Space Clients. A Core is
/// neither: it is an independently authenticated management controller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ClientMode {
    User,
    Space,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Claim {
    Admin {
        #[serde(rename = "userIds")]
        user_ids: BTreeSet<String>,
    },
    Space {
        #[serde(rename = "tenantId")]
        tenant_id: String,
        #[serde(rename = "spaceId")]
        space_id: String,
    },
}

/// The adapter must classify the actual action, not just its HTTP method. An
/// installation plan writes durable state and is a ManageRuntime operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    Read,
    ManageRuntime,
    ManageAdministrators,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    InvalidState,
    ModeChanged,
    Unclaimed,
    BindingMismatch,
    PrincipalMismatch,
    NotAdministrator,
    ReadOnlyClient,
    PermissionMissing,
    AlreadyClaimed,
    LastAdministrator,
}

impl std::fmt::Display for Denied {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Host authorization: {self:?}")
    }
}
impl std::error::Error for Denied {}

/// Persist this state before acknowledging a claim. `generation` changes on an
/// explicit reset, not on access refresh, restart or administrator additions.
/// Claim reservations and receipts belong to the adapter's durable transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostPolicy {
    host_id: String,
    env: RuntimeEnv,
    mode: HostMode,
    generation: String,
    claim: Option<Claim>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && value.len() <= 256
}

impl HostPolicy {
    pub fn unclaimed(
        host_id: String,
        env: RuntimeEnv,
        mode: HostMode,
        generation: String,
    ) -> Result<Self, Denied> {
        let state = Self {
            host_id,
            env,
            mode,
            generation,
            claim: None,
        };
        state.validate(mode)?;
        Ok(state)
    }

    /// Check against the mode in the installed package on every load. A package
    /// upgrade cannot reinterpret an existing binding as a different mode.
    pub fn validate(&self, installed_mode: HostMode) -> Result<(), Denied> {
        if self.mode != installed_mode {
            return Err(Denied::ModeChanged);
        }
        if !valid_id(&self.host_id) || !valid_id(&self.generation) {
            return Err(Denied::InvalidState);
        }
        match (&self.mode, &self.claim) {
            (_, None) => Ok(()),
            (HostMode::Admin, Some(Claim::Admin { user_ids }))
                if !user_ids.is_empty() && user_ids.iter().all(|id| valid_id(id)) =>
            {
                Ok(())
            }
            (
                HostMode::Space,
                Some(Claim::Space {
                    tenant_id,
                    space_id,
                }),
            ) if valid_id(tenant_id) && valid_id(space_id) => Ok(()),
            _ => Err(Denied::InvalidState),
        }
    }

    pub fn mode(&self) -> HostMode {
        self.mode
    }

    pub fn generation(&self) -> &str {
        &self.generation
    }

    pub fn claim(&self) -> Option<&Claim> {
        self.claim.as_ref()
    }

    /// Only call with the subject of a cryptographically verified first-claim
    /// approval after checking its target, expiry, reservation and single use.
    /// The store must serialize the check and persistence with concurrent claims.
    pub fn claim_verified(&mut self, principal: &Principal) -> Result<(), Denied> {
        self.validate(self.mode)?;
        let claim = match (self.mode, principal) {
            (HostMode::Admin, Principal::User { user_id }) if valid_id(user_id) => Claim::Admin {
                user_ids: BTreeSet::from([user_id.clone()]),
            },
            (
                HostMode::Space,
                Principal::Space {
                    tenant_id,
                    space_id,
                },
            ) if valid_id(tenant_id) && valid_id(space_id) => Claim::Space {
                tenant_id: tenant_id.clone(),
                space_id: space_id.clone(),
            },
            _ => return Err(Denied::PrincipalMismatch),
        };
        if let Some(existing) = &self.claim {
            let same = match (existing, principal) {
                (Claim::Admin { user_ids }, Principal::User { user_id }) => {
                    user_ids.contains(user_id)
                }
                _ => existing == &claim,
            };
            return if same {
                Ok(())
            } else {
                Err(Denied::AlreadyClaimed)
            };
        }
        self.claim = Some(claim);
        Ok(())
    }

    /// Run after token/session verification and before dispatch, including
    /// mediated operations. Core adapters must additionally enforce Owner for
    /// human requests to Space Hosts; a Space Client may not borrow Core tokens.
    pub fn authorize(
        &self,
        binding: &Binding,
        granted: &BTreeSet<Permission>,
        action: Permission,
    ) -> Result<(), Denied> {
        self.validate(self.mode)?;
        if binding.host_id != self.host_id
            || binding.env != self.env
            || binding.binding_generation != self.generation
            || binding.validate().is_err()
        {
            return Err(Denied::BindingMismatch);
        }
        match (
            self.claim.as_ref().ok_or(Denied::Unclaimed)?,
            &binding.principal,
            binding.controller_kind,
        ) {
            (
                Claim::Admin { user_ids },
                Principal::User { user_id },
                ControllerKind::UserClient | ControllerKind::Core,
            ) => {
                if !user_ids.contains(user_id) {
                    return Err(Denied::NotAdministrator);
                }
            }
            (
                Claim::Space {
                    tenant_id,
                    space_id,
                },
                Principal::Space {
                    tenant_id: caller_tenant,
                    space_id: caller_space,
                },
                ControllerKind::SpaceClient | ControllerKind::Core,
            ) if tenant_id == caller_tenant && space_id == caller_space => {
                if binding.controller_kind == ControllerKind::SpaceClient
                    && action != Permission::Read
                {
                    return Err(Denied::ReadOnlyClient);
                }
                if action == Permission::ManageAdministrators {
                    return Err(Denied::PrincipalMismatch);
                }
            }
            _ => return Err(Denied::PrincipalMismatch),
        }
        if !granted.contains(&action) {
            return Err(Denied::PermissionMissing);
        }
        Ok(())
    }

    /// The adapter verifies existing-admin consent OR a protected local
    /// authorization transaction, plus the target user's identity, before this.
    pub fn add_verified_administrator(&mut self, user_id: String) -> Result<(), Denied> {
        self.validate(self.mode)?;
        if !valid_id(&user_id) {
            return Err(Denied::InvalidState);
        }
        match self.claim.as_mut() {
            Some(Claim::Admin { user_ids }) => {
                user_ids.insert(user_id);
                Ok(())
            }
            None => Err(Denied::Unclaimed),
            _ => Err(Denied::PrincipalMismatch),
        }
    }

    /// Removing the final admin must not reopen first-claim. Reset is a separate
    /// authorized transaction which also replaces the binding generation.
    pub fn remove_verified_administrator(&mut self, user_id: &str) -> Result<(), Denied> {
        self.validate(self.mode)?;
        match self.claim.as_mut() {
            Some(Claim::Admin { user_ids }) => {
                if user_ids.len() == 1 && user_ids.contains(user_id) {
                    return Err(Denied::LastAdministrator);
                }
                user_ids.remove(user_id);
                Ok(())
            }
            None => Err(Denied::Unclaimed),
            _ => Err(Denied::PrincipalMismatch),
        }
    }
}
