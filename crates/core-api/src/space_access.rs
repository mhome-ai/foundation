//! Space business authorization after credential verification.
//!
//! This is a pure decision: adapters supply a verified principal and current
//! membership facts. No request payload, model output, or transport label may
//! supply those facts. Host administration and personal resources retain their
//! own authorization boundaries.

use crate::client_identity::ClientPrincipal;
use serde::{Deserialize, Serialize};

/// Ordered permissions within one Space, not global account or Host roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SpaceAccessLevel {
    Read,
    Write,
    Manage,
}

impl SpaceAccessLevel {
    pub const fn allows(self, required: Self) -> bool {
        self as u8 >= required as u8
    }
}

/// Only accepted, current memberships grant business permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SpaceMemberRole {
    Member,
    Owner,
}

impl SpaceMemberRole {
    pub const fn access_level(self) -> SpaceAccessLevel {
        match self {
            Self::Member => SpaceAccessLevel::Write,
            Self::Owner => SpaceAccessLevel::Manage,
        }
    }
}

/// Facts from the authenticated ingress and its membership store/cache.
/// A missing ceiling means no additional restriction; it never grants access.
pub struct SpaceAccessFacts<'a> {
    pub principal: &'a ClientPrincipal,
    pub tenant_id: &'a str,
    pub scope_id: &'a str,
    pub member_role: Option<SpaceMemberRole>,
    pub ceiling: Option<SpaceAccessLevel>,
}

impl SpaceAccessFacts<'_> {
    /// Derives permissions without looking up the enrolling user of a Space
    /// Client. Supplying an OWNER role cannot promote a Space Client to MANAGE.
    pub fn effective_level(&self) -> Option<SpaceAccessLevel> {
        if self.tenant_id.trim().is_empty() || self.scope_id.trim().is_empty() {
            return None;
        }
        let level = match self.principal {
            ClientPrincipal::User { user_id } if !user_id.trim().is_empty() => {
                self.member_role?.access_level()
            }
            ClientPrincipal::Space {
                tenant_id,
                scope_id,
            } if tenant_id == self.tenant_id && scope_id == self.scope_id => {
                SpaceAccessLevel::Write
            }
            _ => return None,
        };
        Some(self.ceiling.map_or(level, |ceiling| level.min(ceiling)))
    }

    pub fn allows(&self, required: SpaceAccessLevel) -> bool {
        self.effective_level()
            .is_some_and(|level| level.allows(required))
    }
}
