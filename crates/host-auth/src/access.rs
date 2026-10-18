//! Validation after cloud signature verification. There is no refresh token,
//! offline lease, Client key enrollment, or Core management identity.
use crate::{policy::HostPolicy, signatures};
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use core_api::host::access::*;
use sha2::{Digest, Sha256};

pub fn valid_secret(value: &str) -> bool {
    value.len() == 43
        && URL_SAFE_NO_PAD
            .decode(value)
            .is_ok_and(|bytes| bytes.len() == 32 && URL_SAFE_NO_PAD.encode(bytes) == value)
}

pub fn exchange_digest(secret: &str) -> Result<String> {
    ensure!(valid_secret(secret), "invalid claim exchange secret");
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(secret.as_bytes())))
}

pub fn validate_lifetime(iat: u64, exp: u64, now: u64) -> Result<()> {
    ensure!(
        iat > 0
            && iat <= now.saturating_add(5)
            && exp > now
            && exp > iat
            && exp - iat <= ACCESS_TTL_SECONDS,
        "Host authorization expired or has invalid lifetime"
    );
    Ok(())
}

fn validate_target(binding: &AccessBinding, status: &HostStatus, user: &str) -> Result<()> {
    ensure!(
        !user.is_empty()
            && user.trim() == user
            && user.len() <= 256
            && binding.user_id == user
            && binding.host_id == status.host_id
            && binding.host_key_id == status.host_key_id
            && binding.env == status.env
            && binding.generation == status.generation,
        "Host authorization target or User mismatch"
    );
    match (status.mode, &binding.space) {
        (HostMode::Admin, None) => {}
        (HostMode::SpaceSelf | HostMode::SpaceDep, Some(space)) => {
            space.validate().map_err(anyhow::Error::msg)?
        }
        _ => anyhow::bail!("Host authorization mode mismatch"),
    }
    Ok(())
}

fn validate_issuer(iss: &str, aud: &str, jti: &str) -> Result<()> {
    ensure!(
        iss == CLOUD_ISSUER
            && aud == ACCESS_AUDIENCE
            && !jti.is_empty()
            && jti.len() <= 128
            && jti.trim() == jti,
        "invalid Host authorization issuer, audience or ID"
    );
    Ok(())
}

pub fn validate_access(
    claims: &AccessClaims,
    status: &HostStatus,
    policy: &HostPolicy,
    action: Permission,
    now: u64,
) -> Result<()> {
    validate_issuer(&claims.iss, &claims.aud, &claims.jti)?;
    validate_lifetime(claims.iat, claims.exp, now)?;
    validate_target(&claims.binding, status, &claims.sub)?;
    policy.authorize(&claims.binding, &claims.permissions, action)?;
    Ok(())
}

/// The caller separately verifies the Host signature and fresh probe binding.
pub fn validate_challenge(challenge: &HostChallenge, now: u64) -> Result<()> {
    validate_lifetime(challenge.iat, challenge.exp, now)?;
    ensure!(valid_secret(&challenge.probe_id), "invalid Host probe");
    ensure!(
        challenge
            .exchange_key_hash
            .as_deref()
            .is_none_or(valid_secret),
        "invalid claim exchange hash"
    );
    ensure!(
        signatures::key_id(&signatures::verifying_key(&challenge.host_public_key)?)
            == challenge.status.host_key_id,
        "Host challenge key mismatch"
    );
    match (challenge.status.mode, &challenge.device_binding) {
        (HostMode::SpaceDep, Some(binding)) => {
            binding.validate().map_err(anyhow::Error::msg)?;
            ensure!(
                binding.env == challenge.status.env
                    && binding.generation == challenge.status.generation
                    && binding
                        .host
                        .as_ref()
                        .is_some_and(|host| host.id == challenge.status.host_id
                            && host.key_id == challenge.status.host_key_id),
                "Host challenge does not match its Client binding"
            );
            let claim = binding.space.clone().map(|space| Claim::Space { space });
            ensure!(
                challenge.status.claim == claim,
                "Host and Client Space binding mismatch"
            );
        }
        (HostMode::Admin | HostMode::SpaceSelf, None) => {}
        _ => anyhow::bail!("Host mode and Client binding mismatch"),
    }
    HostPolicy::validate_status(&challenge.status)?;
    Ok(())
}

pub fn validate_claim(
    claims: &ClaimClaims,
    status: &HostStatus,
    challenge: &HostChallenge,
    exchange_key: &str,
    now: u64,
) -> Result<()> {
    validate_issuer(&claims.iss, &claims.aud, &claims.jti)?;
    validate_lifetime(claims.iat, claims.exp, now)?;
    validate_target(&claims.binding, status, &claims.sub)?;
    validate_profile(&claims.display_name, &claims.email)?;
    validate_challenge(challenge, now)?;
    let original = &challenge.status;
    ensure!(
        original.host_id == status.host_id
            && original.host_key_id == status.host_key_id
            && original.env == status.env
            && original.mode == status.mode
            && original.generation == status.generation
            && challenge.exchange_key_hash.as_deref() == Some(&claims.exchange_key_hash)
            && exchange_digest(exchange_key)? == claims.exchange_key_hash,
        "claim does not match the direct Client exchange"
    );
    // Both the observed and current binding must permit this exact User/Space.
    // The Host repeats the current-policy check under its durable mutation lock.
    for observed in [original, status] {
        match (&observed.claim, &claims.binding.space) {
            (None, _) => {}
            (Some(Claim::Admin { user_ids }), None) if user_ids.contains(&claims.sub) => {}
            (Some(Claim::Space { space }), Some(target)) if space == target => {}
            _ => anyhow::bail!("Host is already claimed by another principal"),
        }
    }
    Ok(())
}

pub fn validate_administrator_applicant(
    claims: &AdministratorApplicantClaims,
    status: &HostStatus,
    now: u64,
) -> Result<()> {
    validate_issuer(&claims.iss, &claims.aud, &claims.jti)?;
    ensure!(
        claims.iat > 0
            && claims.iat <= now.saturating_add(5)
            && claims.exp > now
            && claims.exp > claims.iat
            && claims.exp - claims.iat <= AUTHORIZATION_TTL_SECONDS,
        "Administrator request expired"
    );
    validate_target(&claims.binding, status, &claims.sub)?;
    ensure!(
        status.mode == HostMode::Admin
            && matches!(status.claim, Some(Claim::Admin { .. }))
            && valid_secret(&claims.jti),
        "Only claimed Admin Hosts accept requests"
    );
    validate_profile(&claims.display_name, &claims.email)?;
    Ok(())
}

fn validate_profile(name: &str, email: &str) -> Result<()> {
    ensure!(
        name.len() <= 256
            && email.len() <= 320
            && !name.chars().any(char::is_control)
            && !email.chars().any(char::is_control),
        "Invalid User profile"
    );
    Ok(())
}
