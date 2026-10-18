//! Host management authorization. Every remote management caller is a User.
//! The cloud verifies current Space ownership; Core only transports the token.

pub use core_api::host::access::{AccessBinding, Claim, HostMode, Permission};
use core_api::{
    runtime_space::{RuntimeSpacePolicy, SpaceIdentity},
    RuntimeEnv,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    InvalidState,
    ModeChanged,
    Unclaimed,
    BindingMismatch,
    PrincipalMismatch,
    NotAdministrator,
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
    pub(crate) fn validate_status(
        status: &core_api::host::access::HostStatus,
    ) -> Result<(), Denied> {
        Self {
            host_id: status.host_id.clone(),
            env: status.env,
            mode: status.mode,
            generation: status.generation.clone(),
            claim: status.claim.clone(),
        }
        .validate(status.mode)
    }

    pub fn unclaimed(
        host_id: String,
        env: RuntimeEnv,
        mode: HostMode,
        generation: String,
    ) -> Result<Self, Denied> {
        let policy = Self {
            host_id,
            env,
            mode,
            generation,
            claim: None,
        };
        policy.validate(mode)?;
        Ok(policy)
    }

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
            (HostMode::SpaceSelf | HostMode::SpaceDep, Some(Claim::Space { space }))
                if space.validate().is_ok() =>
            {
                Ok(())
            }
            _ => Err(Denied::InvalidState),
        }
    }

    /// Construct an effective policy from the Client's canonical file. This
    /// value is never persisted in Host state.
    pub fn from_device_binding(
        binding: &core_api::device_binding::DeviceBinding,
    ) -> Result<Self, Denied> {
        binding.validate().map_err(|_| Denied::InvalidState)?;
        let host = binding.host.as_ref().ok_or(Denied::InvalidState)?;
        let value = Self {
            host_id: host.id.clone(),
            env: binding.env,
            mode: HostMode::SpaceDep,
            generation: binding.generation.clone(),
            claim: binding.space.clone().map(|space| Claim::Space { space }),
        };
        value.validate(HostMode::SpaceDep)?;
        Ok(value)
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

    pub fn status(&self, host_key_id: String) -> core_api::host::access::HostStatus {
        core_api::host::access::HostStatus {
            host_id: self.host_id.clone(),
            host_key_id,
            env: self.env,
            mode: self.mode,
            generation: self.generation.clone(),
            claim: self.claim.clone(),
        }
    }

    /// No effective policy means a single-Space Host is awaiting first claim.
    /// Services must wait; it must never be interpreted as unrestricted.
    pub fn runtime_policy(&self) -> Result<Option<RuntimeSpacePolicy>, Denied> {
        self.validate(self.mode)?;
        match (&self.mode, &self.claim) {
            (HostMode::Admin, _) => Ok(Some(RuntimeSpacePolicy::Multi {})),
            (HostMode::SpaceSelf | HostMode::SpaceDep, Some(Claim::Space { space })) => {
                Ok(Some(RuntimeSpacePolicy::Single {
                    space: space.clone(),
                }))
            }
            (HostMode::SpaceSelf | HostMode::SpaceDep, None) => Ok(None),
            _ => Err(Denied::InvalidState),
        }
    }

    /// The adapter verifies cloud approval, target identity, current generation,
    /// direct exchange binding and Client admission before persisting this change.
    /// A fresh attempt may confirm the same binding, but never replace it or add
    /// an administrator. Repeating onboarding does not depend on an old receipt.
    pub fn claim_verified(
        &mut self,
        user_id: &str,
        space: Option<&SpaceIdentity>,
    ) -> Result<(), Denied> {
        self.validate(self.mode)?;
        if self.mode == HostMode::SpaceDep || !valid_id(user_id) {
            return Err(Denied::PrincipalMismatch);
        }
        if let Some(claim) = &self.claim {
            return match (claim, space) {
                (Claim::Admin { user_ids }, None) if user_ids.contains(user_id) => Ok(()),
                (Claim::Space { space: current }, Some(target)) if current == target => Ok(()),
                _ => Err(Denied::AlreadyClaimed),
            };
        }
        self.claim = Some(match (self.mode, space) {
            (HostMode::Admin, None) => Claim::Admin {
                user_ids: BTreeSet::from([user_id.to_string()]),
            },
            (HostMode::SpaceSelf, Some(space)) if space.validate().is_ok() => Claim::Space {
                space: space.clone(),
            },
            _ => return Err(Denied::PrincipalMismatch),
        });
        Ok(())
    }

    /// Recheck current local ACL on every request, after cryptographic token
    /// verification. A cached token never freezes administrator membership.
    pub fn authorize(
        &self,
        binding: &AccessBinding,
        granted: &BTreeSet<Permission>,
        action: Permission,
    ) -> Result<(), Denied> {
        self.validate(self.mode)?;
        if binding.host_id != self.host_id
            || binding.env != self.env
            || binding.generation != self.generation
            || !valid_id(&binding.user_id)
        {
            return Err(Denied::BindingMismatch);
        }
        match (
            self.claim.as_ref().ok_or(Denied::Unclaimed)?,
            &binding.space,
        ) {
            (Claim::Admin { user_ids }, None) => {
                if !user_ids.contains(&binding.user_id) {
                    return Err(Denied::NotAdministrator);
                }
            }
            (Claim::Space { space }, Some(target)) if space == target => {
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
