//! D2 lifecycle rules for future Host bearer-token adapters.
//!
//! This module neither verifies grants nor sends HTTP requests. The authority
//! adapter must verify the issuer, audience, subject, controller, binding
//! generation, grant expiry and single use before supplying an online lease.
//! The Host persists `Session` atomically alongside token hashes. Refresh tokens
//! remain fixed; access-token issuance and lookup belong to that adapter.
//!
//! All UTC arguments are trusted Host/OS seconds, not a request's timestamp.
//! Persisted deadlines survive restarts. A clock without a credible UTC anchor
//! must be rejected by the adapter; a plausible integer alone cannot prove time.

use core_api::RuntimeEnv;
use serde::{Deserialize, Serialize};

pub mod renewal;

pub const ACCESS_TTL_SECONDS: u64 = 60 * 60;
pub const ACCESS_REFRESH_AHEAD_SECONDS: u64 = 5 * 60;
pub const AUTHORIZATION_REVIEW_SECONDS: u64 = 24 * 60 * 60;
pub const OFFLINE_AUTHORIZATION_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const REFRESH_IDLE_SECONDS: u64 = 30 * 24 * 60 * 60;

/// Authorization identity, independently of whether a Client or Core calls.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum Principal {
    User {
        #[serde(rename = "userId")]
        user_id: String,
    },
    Space {
        #[serde(rename = "tenantId")]
        tenant_id: String,
        #[serde(rename = "spaceId")]
        space_id: String,
    },
}

/// Established from verified credentials, never from self-declared request
/// metadata. Personal Pods use UserClient; shared Pods use SpaceClient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ControllerKind {
    UserClient,
    SpaceClient,
    Core,
}

/// Immutable session context. This is authenticated metadata, never client input
/// accepted on its own. Permissions remain the authority adapter's responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Binding {
    pub host_id: String,
    pub env: RuntimeEnv,
    pub binding_generation: String,
    pub controller_id: String,
    pub controller_kind: ControllerKind,
    pub principal: Principal,
}

impl Binding {
    pub(crate) fn validate(&self) -> Result<(), Failure> {
        let valid = |s: &str| !s.is_empty() && s.trim() == s && s.len() <= 256;
        let principal_valid = match &self.principal {
            Principal::User { user_id } => valid(user_id),
            Principal::Space {
                tenant_id,
                space_id,
            } => valid(tenant_id) && valid(space_id),
        };
        let controller_matches = matches!(
            (&self.principal, self.controller_kind),
            (
                Principal::User { .. },
                ControllerKind::UserClient | ControllerKind::Core
            ) | (
                Principal::Space { .. },
                ControllerKind::SpaceClient | ControllerKind::Core
            )
        );
        if controller_matches
            && valid(&self.host_id)
            && valid(&self.binding_generation)
            && valid(&self.controller_id)
            && principal_valid
        {
            Ok(())
        } else {
            Err(Failure::BindingMismatch)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    BindingMismatch,
    InvalidLease,
    StaleLease,
    ClockInvalid,
    Revoked,
    SessionExpired,
    AuthorizationExpired,
    AccessExpired,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Host session lifecycle: {self:?}")
    }
}
impl std::error::Error for Failure {}

/// Verified online authority, or a delegated lease bounded by that authority.
/// The exchange proof's five-minute redemption expiry is NOT `authority_until`.
/// That argument is the signed authorization deadline of the upstream authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationLease {
    confirmed_at: u64,
    valid_until: u64,
}

impl AuthorizationLease {
    /// Call only after checking a fresh, authenticated authorization proof.
    /// A delegated Hub must preserve its upstream confirmation time and bound;
    /// it cannot substitute `now` to renew itself while offline.
    pub fn from_verified_authority(
        confirmed_at: u64,
        authority_until: u64,
        now: u64,
    ) -> Result<Self, Failure> {
        let maximum = confirmed_at
            .checked_add(OFFLINE_AUTHORIZATION_SECONDS)
            .ok_or(Failure::InvalidLease)?;
        let lease = Self {
            confirmed_at,
            valid_until: authority_until.min(maximum),
        };
        lease.validate(now)?;
        Ok(lease)
    }

    fn validate(&self, now: u64) -> Result<(), Failure> {
        if self.confirmed_at == 0
            || self.valid_until <= self.confirmed_at
            || self.valid_until - self.confirmed_at > OFFLINE_AUTHORIZATION_SECONDS
        {
            return Err(Failure::InvalidLease);
        }
        if now < self.confirmed_at {
            return Err(Failure::ClockInvalid);
        }
        if now >= self.valid_until {
            return Err(Failure::AuthorizationExpired);
        }
        Ok(())
    }

    pub fn confirmed_at(&self) -> u64 {
        self.confirmed_at
    }

    pub fn valid_until(&self) -> u64 {
        self.valid_until
    }

    pub fn review_due(&self, now: u64) -> bool {
        now >= self.valid_until
            || now.saturating_sub(self.confirmed_at) >= AUTHORIZATION_REVIEW_SECONDS
    }
}

/// Durable Host-side lifecycle metadata; contains no bearer credentials.
/// Loading this value must not reset its deadlines or last-refresh timestamp.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Session {
    binding: Binding,
    session_id: String,
    lease: AuthorizationLease,
    last_refreshed_at: u64,
    revoked: bool,
}

/// Stored with each access-token hash. Do not extend existing access tokens when
/// refreshing a session; their individual expiry remains fixed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccessWindow {
    issued_at: u64,
    expires_at: u64,
}

impl AccessWindow {
    pub fn issued_at(&self) -> u64 {
        self.issued_at
    }

    pub fn expires_at(&self) -> u64 {
        self.expires_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessAction {
    Ready,
    RefreshAccess,
    RenewAuthorization,
    /// Use an independently valid upstream identity, never an expired Host RT.
    EstablishSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Maintenance {
    pub access: AccessAction,
    /// Run independently of LAN refresh; WAN failure must not block usable LAN
    /// credentials or prevent their refresh within the existing offline lease.
    pub review_in_background: bool,
}

impl Session {
    /// Creating a session requires newly verified upstream authority. Callers
    /// must also issue new token material and enforce grants/revocation rules.
    pub fn establish(
        binding: Binding,
        session_id: String,
        lease: AuthorizationLease,
        now: u64,
    ) -> Result<(Self, AccessWindow), Failure> {
        binding.validate()?;
        if session_id.trim().is_empty() || session_id.len() > 256 {
            return Err(Failure::BindingMismatch);
        }
        lease.validate(now)?;
        let session = Self {
            binding,
            session_id,
            lease,
            last_refreshed_at: now,
            revoked: false,
        };
        let access = session.new_access_window(now)?;
        Ok((session, access))
    }

    pub fn binding(&self) -> &Binding {
        &self.binding
    }

    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn lease(&self) -> AuthorizationLease {
        self.lease
    }

    pub fn idle_expires_at(&self) -> Result<u64, Failure> {
        self.last_refreshed_at
            .checked_add(REFRESH_IDLE_SECONDS)
            .ok_or(Failure::ClockInvalid)
    }

    fn active(&self, binding: &Binding, now: u64) -> Result<(), Failure> {
        self.binding.validate()?;
        if binding != &self.binding {
            return Err(Failure::BindingMismatch);
        }
        if self.revoked {
            return Err(Failure::Revoked);
        }
        if self.last_refreshed_at == 0
            || now < self.last_refreshed_at
            || now < self.lease.confirmed_at
        {
            return Err(Failure::ClockInvalid);
        }
        if now >= self.idle_expires_at()? {
            return Err(Failure::SessionExpired);
        }
        Ok(())
    }

    fn new_access_window(&self, now: u64) -> Result<AccessWindow, Failure> {
        let expires_at = now
            .checked_add(ACCESS_TTL_SECONDS)
            .ok_or(Failure::ClockInvalid)?
            .min(self.lease.valid_until);
        Ok(AccessWindow {
            issued_at: now,
            expires_at,
        })
    }

    /// The caller verifies the fixed refresh token against its stored hash and
    /// commits this mutation AND the new access-token hash in one transaction.
    /// Failure must neither touch the idle timer nor discard the refresh token.
    pub fn refresh_access(&mut self, binding: &Binding, now: u64) -> Result<AccessWindow, Failure> {
        self.active(binding, now)?;
        self.lease.validate(now)?;
        let access = self.new_access_window(now)?;
        self.last_refreshed_at = now;
        Ok(access)
    }

    /// Renewal can recover an expired seven-day lease, but cannot revive an
    /// idle-expired or revoked session. It does not touch the idle timer or any
    /// existing access-token expiry. The verified proof must bind this session.
    pub fn renew_authorization(
        &mut self,
        binding: &Binding,
        lease: AuthorizationLease,
        now: u64,
    ) -> Result<(), Failure> {
        self.active(binding, now)?;
        lease.validate(now)?;
        if lease.confirmed_at < self.lease.confirmed_at
            || (lease.confirmed_at == self.lease.confirmed_at && lease != self.lease)
        {
            return Err(Failure::StaleLease);
        }
        self.lease = lease;
        Ok(())
    }

    /// Run after looking up the access-token hash and its session ID. An access
    /// window alone is not a credential and is not sufficient to authorize.
    pub fn authorize(
        &self,
        binding: &Binding,
        access: &AccessWindow,
        now: u64,
    ) -> Result<(), Failure> {
        self.active(binding, now)?;
        self.lease.validate(now)?;
        if now < access.issued_at {
            return Err(Failure::ClockInvalid);
        }
        if access.expires_at <= access.issued_at
            || access.expires_at - access.issued_at > ACCESS_TTL_SECONDS
            || now >= access.expires_at
        {
            return Err(Failure::AccessExpired);
        }
        Ok(())
    }

    /// Call on startup, foreground, connectivity recovery, a timer and before
    /// management dispatch. This computes work; it never performs network I/O.
    pub fn maintenance(
        &self,
        binding: &Binding,
        access: Option<&AccessWindow>,
        now: u64,
    ) -> Result<Maintenance, Failure> {
        match self.active(binding, now) {
            Err(Failure::SessionExpired) => {
                return Ok(Maintenance {
                    access: AccessAction::EstablishSession,
                    review_in_background: false,
                });
            }
            other => other?,
        }
        match self.lease.validate(now) {
            Err(Failure::AuthorizationExpired) => {
                return Ok(Maintenance {
                    access: AccessAction::RenewAuthorization,
                    review_in_background: false,
                });
            }
            other => other?,
        }
        let ready = access.is_some_and(|access| {
            self.authorize(binding, access, now).is_ok()
                && (access.expires_at.saturating_sub(now) > ACCESS_REFRESH_AHEAD_SECONDS
                    // A local refresh cannot improve this deadline. Keep the
                    // valid token while the independent online review runs.
                    || access.expires_at == self.lease.valid_until)
        });
        Ok(Maintenance {
            access: if ready {
                AccessAction::Ready
            } else {
                AccessAction::RefreshAccess
            },
            review_in_background: self.lease.review_due(now),
        })
    }

    /// Token refresh and online renewal both consult this persisted flag.
    pub fn revoke(&mut self) {
        self.revoked = true;
    }
}
