//! Client protocol decisions shared by native and Hub-held identities.
//! Each owner supplies its own key, trusted Host pins, storage and cloud grant source.
use crate::signatures::*;
use anyhow::{ensure, Context, Result};
use core_api::host::auth::{
    AuthError, EnrollmentChallenge, ErrorCode, Grant, HostContext, PublicKey, PROTOCOL,
    REQUEST_WINDOW_SECONDS,
};
use http::{HeaderMap, StatusCode};
use p256::ecdsa::SigningKey;
use std::{collections::BTreeMap, time::Instant};

pub const MAX_ATTEMPTS: usize = 2;
pub const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refresh {
    Context,
    Enrollment,
}

#[derive(Clone)]
pub struct Session {
    pub context: HostContext,
    observed: Instant,
    revision: u64,
    enrollment_revision: u64,
    pub generation: u64,
}

impl Session {
    /// A context-only refresh must never satisfy a waiter needing re-enrollment.
    pub fn satisfies(&self, observed: &Self, refresh: Refresh) -> bool {
        self.generation == observed.generation
            && self.context.host_id == observed.context.host_id
            && match refresh {
                Refresh::Context => self.revision > observed.revision,
                Refresh::Enrollment => self.enrollment_revision > observed.enrollment_revision,
            }
    }

    pub fn server_time(&self) -> Result<u64> {
        self.context
            .server_time
            .checked_add(self.observed.elapsed().as_secs())
            .context("Invalid Host clock")
    }
}

/// One cache per signing identity. The caller serializes refreshes per Host and
/// checks its persistent identity generation before publishing or dispatching.
#[derive(Default)]
pub struct Sessions {
    generation: u64,
    revision: u64,
    hosts: BTreeMap<String, Session>,
}

impl Sessions {
    pub fn get(&self, host_id: &str, generation: u64) -> Option<Session> {
        self.hosts
            .get(host_id)
            .filter(|session| session.generation == generation)
            .cloned()
    }

    pub fn publish(
        &mut self,
        host_id: &str,
        generation: u64,
        context: HostContext,
        enrolled: bool,
    ) -> Result<Session> {
        ensure!(
            generation >= self.generation,
            "Stale Host context publication"
        );
        ensure!(
            context.host_id == host_id
                && context.protocol == PROTOCOL
                && !context.boot_id.is_empty(),
            "Host context identity mismatch"
        );
        self.retire(generation);
        self.revision = self
            .revision
            .checked_add(1)
            .context("Host revision exhausted")?;
        let enrollment_revision = if enrolled {
            self.revision
        } else {
            self.hosts
                .get(host_id)
                .filter(|s| s.context.boot_id == context.boot_id)
                .map(|s| s.enrollment_revision)
                .unwrap_or(0)
        };
        let session = Session {
            context,
            observed: Instant::now(),
            revision: self.revision,
            enrollment_revision,
            generation,
        };
        self.hosts.insert(host_id.into(), session.clone());
        Ok(session)
    }

    pub fn retire(&mut self, generation: u64) {
        self.generation = self.generation.max(generation);
        self.hosts
            .retain(|_, session| session.generation >= self.generation);
    }
}

pub fn verify_context(
    proof: &str,
    host_key: &PublicKey,
    host_id: &str,
    probe_id: &str,
) -> Result<HostContext> {
    let context: HostContext = verify_jws(&verifying_key(host_key)?, "meow-host-context", proof)?;
    ensure!(
        context.host_id == host_id
            && context.probe_id == probe_id
            && context.protocol == PROTOCOL
            && !context.boot_id.is_empty(),
        "Host context identity mismatch"
    );
    Ok(context)
}

pub fn verify_enrollment_challenge(
    proof: &str,
    host_key: &PublicKey,
    host_id: &str,
    user_id: &str,
    client_key: &PublicKey,
    probe_id: &str,
) -> Result<EnrollmentChallenge> {
    let challenge: EnrollmentChallenge = verify_jws(
        &verifying_key(host_key)?,
        "meow-host-enrollment-challenge",
        proof,
    )?;
    ensure!(
        challenge.host_id == host_id
            && challenge.user_id == user_id
            && &challenge.client_public_key == client_key
            && challenge.probe_id == probe_id
            && !challenge.boot_id.is_empty(),
        "Host enrollment challenge mismatch"
    );
    Ok(challenge)
}

/// ONLY for a grant returned directly by the configured, authenticated HTTPS cloud.
/// This parses the trusted response, not a LAN-supplied token. The cloud verified
/// the challenge against its user/Host directory. This lets a Hub bootstrap a pin
/// through its existing enrollment endpoint without impersonating a user login.
pub fn cloud_confirmed_enrollment(
    grant: &str,
    challenge: &str,
    host_id: &str,
    user_id: &str,
    client_key: &PublicKey,
    probe_id: &str,
) -> Result<(PublicKey, EnrollmentChallenge)> {
    let grant: Grant = serde_json::from_value(jws_payload_json(grant)?)?;
    ensure!(
        grant.aud == "meow-host-client-registration"
            && grant.sub == user_id
            && grant.host_id == host_id
            && grant.client_public_key.as_ref() == Some(client_key),
        "Cloud Host registration identity mismatch"
    );
    let challenge = verify_enrollment_challenge(
        challenge,
        &grant.host_public_key,
        host_id,
        user_id,
        client_key,
        probe_id,
    )?;
    ensure!(
        grant.challenge == challenge.challenge
            && grant.boot_id.as_deref() == Some(challenge.boot_id.as_str()),
        "Cloud Host registration challenge mismatch"
    );
    Ok((grant.host_public_key, challenge))
}

pub struct Exchange {
    pub headers: HeaderMap,
    identity: RequestIdentity,
    response: ResponseIdentity,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Refresh(Refresh),
    Rejected(Option<ErrorCode>),
}

impl Exchange {
    pub fn sign(
        key: &SigningKey,
        user_id: &str,
        session: &Session,
        method: &str,
        uri: &str,
        body: &[u8],
    ) -> Result<Self> {
        let created = session.server_time()?;
        let identity = RequestIdentity {
            user_id: user_id.into(),
            host_id: session.context.host_id.clone(),
            client_key_id: key_id(key.verifying_key()),
            boot_id: session.context.boot_id.clone(),
            request_id: random_id(),
            created,
            expires: created
                .checked_add(REQUEST_WINDOW_SECONDS)
                .context("Invalid Host clock")?,
        };
        let headers = sign_request(key, method, uri, body, &identity)?;
        let response = ResponseIdentity {
            host_id: identity.host_id.clone(),
            request_digest: request_digest(&headers, method, uri)?,
        };
        Ok(Self {
            headers,
            identity,
            response,
        })
    }

    /// No network error enters this method: ambiguous failures cannot be retried.
    /// Only a verified rejection before dispatch can request one recovery/retry.
    pub fn verify(
        &self,
        host_key: &PublicKey,
        status: StatusCode,
        headers: &HeaderMap,
        body: &[u8],
        attempt: usize,
    ) -> Result<Outcome> {
        verify_response(
            headers,
            &verifying_key(host_key)?,
            status.as_u16(),
            body,
            &self.response,
        )?;
        if status.is_success() {
            return Ok(Outcome::Success);
        }
        if status == StatusCode::UNAUTHORIZED {
            let error: AuthError = serde_json::from_slice(body)?;
            ensure!(
                error.host_id == self.identity.host_id
                    && error.request_id == self.identity.request_id,
                "Host authorization error does not match request"
            );
            if attempt < MAX_ATTEMPTS - 1 {
                match error.code {
                    ErrorCode::HostReauthRequired | ErrorCode::HostEnrollmentRequired => {
                        return Ok(Outcome::Refresh(Refresh::Enrollment));
                    }
                    ErrorCode::HostContextChanged => return Ok(Outcome::Refresh(Refresh::Context)),
                    _ => {}
                }
            }
            return Ok(Outcome::Rejected(Some(error.code)));
        }
        Ok(Outcome::Rejected(None))
    }
}
