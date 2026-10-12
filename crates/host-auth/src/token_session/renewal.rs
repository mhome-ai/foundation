//! Per-owner, per-Host renewal coordination, independent of the HTTP runtime.
//!
//! Drive this coordinator from a timer/startup/foreground/connectivity event and
//! `Session::maintenance`. LAN refresh and online review have separate lanes:
//! a slow cloud request must not block local refresh. Never enqueue or replay a
//! management mutation here. Only authentication maintenance belongs here.

use anyhow::{ensure, Result};
use rand::Rng;
use std::collections::BTreeMap;

pub const ATTEMPT_TIMEOUT_MS: u64 = 30_000;
pub const RETRY_MIN_MS: u64 = 5_000;
pub const RETRY_MAX_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Work {
    RefreshAccess,
    RenewAuthorization,
    EstablishSession,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureKind {
    /// Transport/server failure: keep credentials and bounded offline rights.
    Transient,
    /// Stop upstream retries until login is restored; existing LAN authorization
    /// remains governed by its lease. An explicit logout instead calls retire.
    LoginRequired,
    /// Authenticated revocation: do not automatically re-enroll around it.
    Revoked,
}

/// Move this ticket into the task. Only its original coordinator can accept it.
/// It contains no token or other authorization credential.
pub struct Attempt {
    owner: [u8; 32],
    sequence: u64,
    work: Work,
}

#[derive(Default)]
struct Lane {
    failures: u32,
    retry_at_ms: u64,
    in_flight: Option<(u64, u64)>,
}

pub struct Coordinator {
    owner: [u8; 32],
    generation: u64,
    sequence: u64,
    clock_ms: u64,
    retired: bool,
    revoked: bool,
    login_required: bool,
    lanes: BTreeMap<Work, Lane>,
}

impl Coordinator {
    /// One instance per login/device owner AND Host, guarded by that owner's
    /// mutex. Persisted generation must increase on logout/account replacement.
    /// A process restart recreates only retry state, not Session deadlines.
    pub fn new(generation: u64) -> Self {
        Self {
            owner: rand::random(),
            generation,
            sequence: 0,
            clock_ms: 0,
            retired: false,
            revoked: false,
            login_required: false,
            lanes: BTreeMap::new(),
        }
    }

    fn observe_clock(&mut self, now_ms: u64) -> Result<()> {
        ensure!(now_ms >= self.clock_ms, "renewal clock moved backwards");
        self.clock_ms = now_ms;
        Ok(())
    }

    /// `now_ms` is elapsed monotonic time, not UTC. Returns None while another
    /// task owns the lane, retry backoff applies, or human attention is needed.
    pub fn begin(&mut self, work: Work, now_ms: u64) -> Result<Option<Attempt>> {
        self.observe_clock(now_ms)?;
        if self.retired || self.revoked || (self.login_required && work != Work::RefreshAccess) {
            return Ok(None);
        }
        let lane = self.lanes.entry(work).or_default();
        if let Some((_, deadline)) = lane.in_flight {
            if now_ms < deadline {
                return Ok(None);
            }
            // A task that never completes must not hold the lane forever. Its
            // ticket is retired before scheduling another attempt.
            lane.in_flight = None;
            retry(lane, now_ms);
        }
        if now_ms < lane.retry_at_ms {
            return Ok(None);
        }
        self.sequence = self
            .sequence
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("renewal sequence exhausted"))?;
        let deadline = now_ms
            .checked_add(ATTEMPT_TIMEOUT_MS)
            .ok_or_else(|| anyhow::anyhow!("renewal clock exhausted"))?;
        lane.in_flight = Some((self.sequence, deadline));
        Ok(Some(Attempt {
            owner: self.owner,
            sequence: self.sequence,
            work,
        }))
    }

    fn accepts(&self, attempt: &Attempt, now_ms: u64) -> bool {
        !self.retired
            && !self.revoked
            && self.owner == attempt.owner
            && self.lanes.get(&attempt.work).is_some_and(|lane| {
                lane.in_flight.is_some_and(|(sequence, deadline)| {
                    sequence == attempt.sequence && now_ms < deadline
                })
            })
    }

    /// Publish under the SAME owner lock that serializes logout. The closure
    /// must atomically check the supplied persisted generation, merge the
    /// response into the latest session (not an old snapshot), and save it.
    /// Recheck binding/session ID and never let a LAN refresh overwrite a newer
    /// online lease. A closure error must leave durable state unchanged.
    ///
    /// Late results never invoke the closure. This API deliberately does not
    /// return a boolean "permission to publish later", which would race logout.
    pub fn succeed<T>(
        &mut self,
        attempt: Attempt,
        now_ms: u64,
        publish: impl FnOnce(u64) -> Result<T>,
    ) -> Result<Option<T>> {
        self.observe_clock(now_ms)?;
        if !self.accepts(&attempt, now_ms) {
            return Ok(None);
        }
        let result = publish(self.generation);
        let lane = self.lanes.get_mut(&attempt.work).unwrap();
        lane.in_flight = None;
        match result {
            Ok(value) => {
                lane.failures = 0;
                lane.retry_at_ms = 0;
                // Replacing a session invalidates all work captured from the
                // old session. Renewal and refresh keep independent lanes.
                if attempt.work == Work::EstablishSession {
                    self.lanes.clear();
                }
                Ok(Some(value))
            }
            Err(error) => {
                retry(lane, now_ms);
                Err(error)
            }
        }
    }

    /// Classify only trusted auth responses as LoginRequired/Revoked. An
    /// unauthenticated LAN error or ordinary network failure cannot revoke.
    /// The owner must also persist revocation in its session/credential store.
    pub fn fail(&mut self, attempt: Attempt, failure: FailureKind, now_ms: u64) -> Result<bool> {
        self.observe_clock(now_ms)?;
        if !self.accepts(&attempt, now_ms) {
            return Ok(false);
        }
        let lane = self.lanes.get_mut(&attempt.work).unwrap();
        lane.in_flight = None;
        match failure {
            FailureKind::Transient => retry(lane, now_ms),
            FailureKind::LoginRequired => {
                self.login_required = true;
                for work in [Work::RenewAuthorization, Work::EstablishSession] {
                    if let Some(lane) = self.lanes.get_mut(&work) {
                        lane.in_flight = None;
                    }
                }
            }
            FailureKind::Revoked => {
                self.revoked = true;
                self.lanes.clear();
            }
        }
        Ok(true)
    }

    /// Use for an actual connectivity restoration, not every failed request.
    /// This bypasses transient backoff once, never an in-flight task or a denial.
    pub fn connection_restored(&mut self, now_ms: u64) -> Result<()> {
        self.observe_clock(now_ms)?;
        for lane in self.lanes.values_mut() {
            lane.retry_at_ms = now_ms;
        }
        Ok(())
    }

    /// Login restored for the SAME owner. Account changes need retire + a new
    /// coordinator/generation. Revocation is not cleared by this method.
    pub fn upstream_identity_restored(&mut self, now_ms: u64) -> Result<()> {
        self.connection_restored(now_ms)?;
        self.login_required = false;
        Ok(())
    }

    pub fn needs_attention(&self) -> bool {
        self.login_required || self.revoked
    }

    /// Synchronously retire before clearing credentials or publishing a login.
    /// A retired coordinator can never be resumed, even with a fresh token.
    pub fn retire(&mut self) {
        self.retired = true;
        self.lanes.clear();
    }
}

fn retry(lane: &mut Lane, now_ms: u64) {
    lane.failures = lane.failures.saturating_add(1);
    let multiplier = 1_u64 << lane.failures.saturating_sub(1).min(6);
    let base = (RETRY_MIN_MS * multiplier).min(RETRY_MAX_MS);
    let jitter = rand::thread_rng().gen_range(0..=base / 4);
    let delay = base.saturating_add(jitter).min(RETRY_MAX_MS);
    lane.retry_at_ms = now_ms.saturating_add(delay);
}
