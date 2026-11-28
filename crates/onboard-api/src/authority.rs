//! Onboarding-specific signed claims. Validation here checks relationships only;
//! it never substitutes for a trusted signature/possession verifier or issuer ACL.
use crate::{hex, id, Environment, Identity, Plan, Product, MAX_SAFE_TIMESTAMP, PROTOCOL_VERSION};
use serde::{Deserialize, Serialize};

pub const OPERATION_ISSUER: &str = "meow-onboard-authority";
pub const OPERATION_AUDIENCE: &str = "meow-product-onboard";
pub const OPERATION_PROOF_TYPE: &str = "meow-onboard-operation+jws";
pub const CHALLENGE_PROOF_TYPE: &str = "meow-onboard-challenge+jws";
pub const POSSESSION_PROOF_TYPE: &str = "meow-onboard-possession+jws";
pub const MAX_GRANT_LIFETIME_MS: u64 = 300_000;
pub const MAX_CHALLENGE_LIFETIME_MS: u64 = 60_000;
pub const MAX_FUTURE_SKEW_MS: u64 = 5_000;

/// A Cloud approval for one exact operation, never a role operating credential.
/// Intentionally no Debug implementation, matching Start's secret-proof boundary.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OperationGrant {
    pub protocol_version: u32,
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub jti: String,
    pub environment: Environment,
    pub product: Identity,
    pub plan: Plan,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

fn lifetime(issued: u64, expires: u64, now: u64, limit: u64) -> Result<(), &'static str> {
    if now == 0
        || now > MAX_SAFE_TIMESTAMP
        || issued == 0
        || issued > now.saturating_add(MAX_FUTURE_SKEW_MS)
        || expires <= now
        || expires > MAX_SAFE_TIMESTAMP
        || expires <= issued
        || expires - issued > limit
    {
        return Err("expired or invalid onboarding authority lifetime");
    }
    Ok(())
}

impl OperationGrant {
    /// Invoke only after validating the signature against configured Cloud keys.
    /// The caller also proves possession and checks current admission policy.
    pub fn validate(
        &self,
        product: &Product,
        plan: &Plan,
        now_ms: u64,
    ) -> Result<(), &'static str> {
        plan.validate(product)?;
        if self.protocol_version != PROTOCOL_VERSION
            || self.iss != OPERATION_ISSUER
            || self.aud != OPERATION_AUDIENCE
            || !id(&self.sub)
            || !hex(&self.jti, 32)
            || self.environment != product.environment
            || self.product != product.identity
            || self.plan != *plan
        {
            return Err("substituted onboarding authority or target");
        }
        lifetime(
            self.issued_at_ms,
            self.expires_at_ms,
            now_ms,
            MAX_GRANT_LIFETIME_MS,
        )
    }
}

/// Public, signed Cloud challenge for initial approval or an explicit user retry.
/// Both nonces are fresh 128-bit random values; this is not a renewal scheduler.
/// The requesting device retains its own nonce only for this live exchange.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorityChallenge {
    pub protocol_version: u32,
    pub iss: String,
    pub aud: String,
    pub environment: Environment,
    pub product: Identity,
    pub plan: Plan,
    pub device_nonce: String,
    pub nonce: String,
    pub issued_at_ms: u64,
    pub expires_at_ms: u64,
}

impl AuthorityChallenge {
    /// Signature verification precedes this check. The issuer separately consumes
    /// its nonce once and checks current User/Space/Hub approval before issuance.
    pub fn validate(
        &self,
        product: &Product,
        plan: &Plan,
        device_nonce: &str,
        now_ms: u64,
    ) -> Result<(), &'static str> {
        plan.validate(product)?;
        if self.protocol_version != PROTOCOL_VERSION
            || self.iss != OPERATION_ISSUER
            || self.aud != OPERATION_AUDIENCE
            || self.environment != product.environment
            || self.product != product.identity
            || self.plan != *plan
            || !hex(device_nonce, 32)
            || self.device_nonce != device_nonce
            || !hex(&self.nonce, 32)
        {
            return Err("substituted onboarding authority challenge");
        }
        lifetime(
            self.issued_at_ms,
            self.expires_at_ms,
            now_ms,
            MAX_CHALLENGE_LIFETIME_MS,
        )
    }
}

/// Root-signed SHA-256 of the exact compact Cloud challenge bytes. It contains
/// no User refresh token and cannot authorize another challenge or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PossessionProof {
    pub protocol_version: u32,
    pub challenge_digest: String,
}

impl PossessionProof {
    pub fn validate(&self, challenge_digest: &str) -> Result<(), &'static str> {
        if self.protocol_version != PROTOCOL_VERSION
            || !hex(challenge_digest, 64)
            || self.challenge_digest != challenge_digest
        {
            return Err("substituted onboarding possession challenge");
        }
        Ok(())
    }
}
