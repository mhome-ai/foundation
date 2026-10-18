//! Native LAN Hub commissioning shared by desktop, Android and iOS.
//! Platform adapters own discovery, account-bound I/O and connection scheduling.
//! This engine owns the protocol, trust checks, deadlines and retry state.
mod proof;
#[cfg(test)]
mod tests;

use app_facade_api::hub as api;
use async_trait::async_trait;
use core_api::{host::auth::PublicKey, runtime_space::RuntimeSpacePolicy, RuntimeEnv};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::Instant;

pub const SETUP_TIMEOUT: Duration = Duration::from_secs(600);
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const RETAIN_FAILURE: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Candidate {
    pub host_id: String,
    pub host_name: String,
    pub hub_type: String,
    pub fingerprint: String,
    pub public_key: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub runtime_space_policy: RuntimeSpacePolicy,
    /// Opaque platform LAN observation; never supplied by the UI.
    pub route_context: String,
}
impl Candidate {
    fn view(&self) -> api::HubCandidate {
        api::HubCandidate {
            host_id: self.host_id.clone(),
            host_name: self.host_name.clone(),
            hub_type: self.hub_type.clone(),
            version: None,
            available: true,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LanResponse {
    pub body: Value,
    /// The key observed in the actual TLS handshake, not a response field.
    pub peer_key: PublicKey,
}

#[async_trait]
pub trait Platform: Send + Sync + 'static {
    async fn account_context(&self) -> Result<String, String>;
    async fn candidates(&self, context: &str, scope: &str) -> Result<Vec<Candidate>, String>;
    async fn view(&self, context: &str, scope: &str) -> Result<api::LocalHubState, String>;
    async fn cloud(
        &self,
        context: &str,
        scope: &str,
        path: &str,
        body: Value,
    ) -> Result<Value, String>;
    /// Sends once, with no redirects/replay/proxy. Select the recorded LAN and
    /// reject network changes. `None` is allowed only for public challenge I/O.
    #[allow(clippy::too_many_arguments)] // Keep the explicit transport and TLS inputs in the platform contract.
    async fn lan(
        &self,
        context: &str,
        candidate: &Candidate,
        base: &str,
        pin: Option<&PublicKey>,
        path: &str,
        body: Value,
        timeout: Duration,
    ) -> Result<LanResponse, String>;
    async fn connected(
        &self,
        context: &str,
        scope: &str,
        hub_id: &str,
        candidate: &Candidate,
    ) -> Result<(), String>;
}

#[derive(Clone)]
struct Session {
    context: String,
    scope: String,
    candidate: Candidate,
    base: String,
    key: PublicKey,
    pairing_code: String,
    deadline: Instant,
}
struct Operation {
    session_id: String,
    session: Session,
    view: api::HubPairingOperation,
    deadline: Instant,
    task: Option<tokio::task::AbortHandle>,
    is_test: bool,
}
#[derive(Default)]
struct State {
    sessions: HashMap<String, Session>,
    operations: HashMap<(String, String), Operation>,
    starts: HashMap<(String, String), u64>,
    sequence: u64,
}
pub struct Commissioning {
    platform: Arc<dyn Platform>,
    state: Mutex<State>,
}
impl Commissioning {
    pub fn new(platform: Arc<dyn Platform>) -> Arc<Self> {
        Arc::new(Self {
            platform,
            state: Mutex::new(State::default()),
        })
    }
    async fn current(&self, expected: &str) -> Result<(), String> {
        if self.platform.account_context().await? != expected {
            return Err("Login changed during Hub setup".into());
        }
        Ok(())
    }
    fn clean(state: &mut State) {
        state.sessions.retain(|_, s| s.deadline > Instant::now());
        state.operations.retain(|_, op| {
            let retain = op.deadline + RETAIN_FAILURE > Instant::now();
            if !retain {
                if let Some(task) = &op.task {
                    task.abort();
                }
            }
            retain
        });
    }
    pub async fn candidates(&self, scope: &str) -> Result<api::HubCandidatesResponse, String> {
        let context = self.platform.account_context().await?;
        let candidates = self.platform.candidates(&context, scope).await?;
        self.current(&context).await?;
        Ok(api::HubCandidatesResponse {
            scope_id: scope.into(),
            candidates: candidates.iter().map(Candidate::view).collect(),
        })
    }
    pub async fn view(&self, scope: &str) -> Result<api::LocalHubState, String> {
        let context = self.platform.account_context().await?;
        let mut view = self.platform.view(&context, scope).await?;
        self.current(&context).await?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hub setup state unavailable")?;
        Self::clean(&mut state);
        if let Some(op) = state
            .operations
            .get(&(context, scope.into()))
            .filter(|op| op.view.state != "COMPLETED")
        {
            view.pairing = Some(op.view.clone());
        }
        Ok(view)
    }
    pub async fn start(
        self: &Arc<Self>,
        scope: &str,
        request: api::HubSetupStartRequest,
    ) -> Result<api::HubSetupStartResponse, String> {
        self.start_pinned(scope, request, None).await
    }
    async fn start_pinned(
        self: &Arc<Self>,
        scope: &str,
        request: api::HubSetupStartRequest,
        expected_host_key: Option<&PublicKey>,
    ) -> Result<api::HubSetupStartResponse, String> {
        if scope.trim().is_empty() || request.host_id.trim().is_empty() {
            return Err("Space and Host are required".into());
        }
        let context = self.platform.account_context().await?;
        let start_key = (context.clone(), scope.to_owned());
        let sequence = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Hub setup state unavailable")?;
            state.sequence += 1;
            let sequence = state.sequence;
            state.starts.insert(start_key.clone(), sequence);
            if let Some(previous) = state.operations.remove(&start_key) {
                if let Some(task) = previous.task {
                    task.abort();
                }
            }
            state
                .sessions
                .retain(|_, s| s.context != context || s.scope != scope);
            sequence
        };
        let current = self.platform.view(&context, scope).await?;
        if current.configured
            && current
                .hub
                .as_ref()
                .is_none_or(|hub| !hub.cloud.status.eq_ignore_ascii_case("setup"))
        {
            return Err("This Space already has a Hub".into());
        }
        if current
            .operation
            .as_ref()
            .is_some_and(|op| op.kind != api::HubOperationKind::Setup)
        {
            return Err("Finish the current Hub operation first".into());
        }
        let mut candidates = self
            .platform
            .candidates(&context, scope)
            .await?
            .into_iter()
            .filter(|c| c.host_id == request.host_id);
        let mut candidate = candidates
            .next()
            .ok_or("Selected Hub is no longer discovered on this LAN")?;
        if candidates.next().is_some() {
            return Err("Hub identity has conflicting LAN observations".into());
        }
        if let Some(endpoint) = request.endpoint {
            // Native adapters must validate any explicit endpoint against their
            // own route policy. Mobile does not offer endpoint overrides.
            candidate.addresses = vec![endpoint.address];
            candidate.port = endpoint.port;
        }
        let (base, key, address) = self.probe(&context, &candidate).await?;
        if expected_host_key.is_some_and(|expected| *expected != key) {
            return Err("Hub TLS identity does not belong to the paired product".into());
        }
        candidate.addresses = vec![address];
        let reply = self
            .lan(
                &context,
                &candidate,
                &base,
                Some(&key),
                "/commission/pairing/start",
                json!({"env":RuntimeEnv::current(),"candidateFingerprint":candidate.fingerprint}),
                READ_TIMEOUT,
            )
            .await?;
        let id = text(&reply.body, "pairingSessionId")?;
        let code = text(&reply.body, "pairingCode")?;
        let ttl = reply.body["expiresIn"]
            .as_u64()
            .filter(|v| *v > 0 && *v <= 600)
            .ok_or("Invalid Hub pairing expiry")?;
        let session = Session {
            context: context.clone(),
            scope: scope.into(),
            candidate: candidate.clone(),
            base,
            key,
            pairing_code: code.clone(),
            deadline: Instant::now() + Duration::from_secs(ttl),
        };
        self.current(&context).await?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hub setup state unavailable")?;
        Self::clean(&mut state);
        if state.starts.get(&start_key) != Some(&sequence) {
            return Err("Hub setup was replaced by a newer attempt".into());
        }
        state.starts.remove(&start_key);
        // An explicit fresh attempt retires old local work. Cloud and Core also
        // fence their attempt IDs; a late callback cannot publish into this one.
        if let Some(previous) = state.operations.remove(&(context.clone(), scope.into())) {
            if let Some(task) = previous.task {
                task.abort();
            }
        }
        state
            .sessions
            .retain(|_, s| s.context != context || s.scope != scope);
        state.sessions.insert(id.clone(), session);
        Ok(api::HubSetupStartResponse {
            scope_id: scope.into(),
            session_id: id,
            candidate: candidate.view(),
            pairing_code: code,
            expires_at: now_ms() + (ttl * 1000) as i64,
        })
    }
    /// Product initialization already authenticated the owner and paired with
    /// this exact Host key. Reuse normal Hub setup without another UI ceremony.
    pub async fn setup_product(
        self: &Arc<Self>,
        scope: &str,
        host_id: &str,
        host_key: &PublicKey,
    ) -> Result<(), String> {
        let context = self.platform.account_context().await?;
        let view = self.platform.view(&context, scope).await?;
        if view.configured
            && view.operation.is_none()
            && view
                .hub
                .as_ref()
                .is_some_and(|h| !h.cloud.status.eq_ignore_ascii_case("setup"))
        {
            return Ok(());
        }
        let deadline = Instant::now() + SETUP_TIMEOUT;
        // The dependent Host can launch its preinstalled Core only after the
        // Client binding commits. Wait for that actual LAN advertisement.
        loop {
            self.current(&context).await?;
            if self
                .platform
                .candidates(&context, scope)
                .await?
                .iter()
                .any(|c| c.host_id == host_id)
            {
                break;
            }
            if Instant::now() >= deadline {
                return Err("The device's Core did not become discoverable".into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        let session = self
            .start_pinned(
                scope,
                api::HubSetupStartRequest {
                    host_id: host_id.into(),
                    endpoint: None,
                },
                Some(host_key),
            )
            .await?;
        self.confirm(scope, &session.session_id).await?;
        loop {
            self.current(&context).await?;
            let state = self.status(scope, &session.session_id).await?;
            if state.complete {
                return Ok(());
            }
            if let Some(failure) = state.pairing.as_ref().and_then(|p| p.failure.as_ref()) {
                return Err(failure.reason.clone());
            }
            if let Some(failure) = state.operation.as_ref().and_then(|p| p.failure.as_ref()) {
                return Err(failure.reason.clone());
            }
            if Instant::now() >= deadline {
                return Err("Device Hub setup timed out; retry initialization".into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    pub async fn confirm(
        self: &Arc<Self>,
        scope: &str,
        id: &str,
    ) -> Result<api::HubSetupConfirmResponse, String> {
        self.confirm_inner(scope, id, false).await
    }
    /// Native test orchestration only; never exposed by the App facade.
    pub async fn confirm_test(
        self: &Arc<Self>,
        scope: &str,
        id: &str,
    ) -> Result<api::HubSetupConfirmResponse, String> {
        self.confirm_inner(scope, id, true).await
    }
    async fn confirm_inner(
        self: &Arc<Self>,
        scope: &str,
        id: &str,
        is_test: bool,
    ) -> Result<api::HubSetupConfirmResponse, String> {
        let context = self.platform.account_context().await?;
        let key = (context.clone(), scope.to_owned());
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Hub setup state unavailable")?;
            Self::clean(&mut state);
            if state
                .operations
                .get(&key)
                .is_some_and(|op| op.session_id == id)
            {
                return Ok(accepted(scope, id));
            }
            let session = state
                .sessions
                .get(id)
                .filter(|s| s.context == context && s.scope == scope)
                .cloned()
                .ok_or("Hub pairing expired; start again")?;
            state.sessions.remove(id);
            let started = now_ms();
            state.operations.insert(
                key.clone(),
                Operation {
                    session_id: id.into(),
                    session,
                    deadline: Instant::now() + SETUP_TIMEOUT,
                    task: None,
                    is_test,
                    view: api::HubPairingOperation {
                        session_id: id.into(),
                        state: "RUNNING".into(),
                        phase: "INITIALIZING".into(),
                        started_at: started,
                        expires_at: started + SETUP_TIMEOUT.as_millis() as i64,
                        host_id: None,
                        hub_id: None,
                        failure: None,
                    },
                },
            );
        }
        let this = self.clone();
        let session_id = id.to_owned();
        let task_key = key.clone();
        let task = tokio::spawn(async move {
            this.run(task_key, session_id).await;
        });
        if let Some(op) = self
            .state
            .lock()
            .map_err(|_| "Hub setup state unavailable")?
            .operations
            .get_mut(&key)
            .filter(|op| op.session_id == id)
        {
            op.task = Some(task.abort_handle());
        }
        Ok(accepted(scope, id))
    }
    fn update(&self, key: &(String, String), id: &str, f: impl FnOnce(&mut Operation)) {
        if let Ok(mut state) = self.state.lock() {
            if let Some(op) = state
                .operations
                .get_mut(key)
                .filter(|op| op.session_id == id)
            {
                f(op);
            }
        }
    }
    async fn run(self: Arc<Self>, key: (String, String), id: String) {
        let Some((session, deadline, is_test)) = self.state.lock().ok().and_then(|s| {
            s.operations
                .get(&key)
                .filter(|op| op.session_id == id)
                .map(|op| (op.session.clone(), op.deadline, op.is_test))
        }) else {
            return;
        };
        self.update(&key, &id, |op| {
            op.view.host_id = Some(session.candidate.host_id.clone())
        });
        let result = tokio::time::timeout_at(deadline, async {
            let nonce = uuid::Uuid::new_v4().to_string();
            let challenge = self.challenge(&session.context,&session.candidate,&session.base,Some(&session.key),&nonce).await?;
            self.update(&key,&id, |op| op.view.phase = "INITIATING_SETUP".into());
            let initiated = self.cloud(&session, "/hub/setup/initiate", json!({"hostId":session.candidate.host_id,"hubType":session.candidate.hub_type,"name":session.candidate.host_name,"candidateFingerprint":session.candidate.fingerprint,"clientNonce":nonce,"isTest":is_test})).await?;
            let hub_id = text(&initiated,"hubId")?;
            self.update(&key,&id, |op| { op.view.hub_id = Some(hub_id); op.view.phase = "STARTING_SETUP".into(); });
            let response = self.lan(&session.context,&session.candidate,&session.base,Some(&session.key),"/setup/start",json!({
                "env":RuntimeEnv::current(),"setupToken":text(&initiated,"setupToken")?,"commissionSessionId":text(&initiated,"commissionSessionId")?,
                "commissionSessionSecret":text(&initiated,"commissionSessionSecret")?,"candidateFingerprint":session.candidate.fingerprint,"clientNonce":nonce,
                "challengeSignature":text(&challenge,"signature")?,"pairingSessionId":id,"pairingCode":session.pairing_code
            }),deadline.saturating_duration_since(Instant::now())).await?;
            if response.body["ok"] != true || response.body["state"] != "completion_pending" { return Err("Invalid Hub setup response".into()); }
            Ok::<(),String>(())
        }).await.unwrap_or_else(|_| Err("Hub setup exceeded its ten-minute deadline".into()));
        self.update(&key, &id, |op| {
            if op.view.state != "RUNNING" {
                return;
            }
            match result {
                Ok(()) => op.view.phase = "WAITING_FOR_CLOUD".into(),
                Err(reason) => {
                    op.view.state = "FAILED".into();
                    op.view.phase = "FAILED".into();
                    op.view.failure = Some(api::HubOperationFailure {
                        reason,
                        at: Some(now_ms()),
                    });
                }
            }
        });
        // A lost response is not proof of failure. Status checks the exact cloud
        // commit; an explicit retry replaces an incomplete attempt immediately.
    }
    pub async fn status(
        &self,
        scope: &str,
        id: &str,
    ) -> Result<api::HubSetupStatusResponse, String> {
        let context = self.platform.account_context().await?;
        let key = (context.clone(), scope.to_owned());
        let (session, mut pairing, deadline) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| "Hub setup state unavailable")?;
            Self::clean(&mut state);
            let op = state
                .operations
                .get(&key)
                .filter(|op| op.session_id == id)
                .ok_or("Hub setup attempt is no longer active")?;
            (op.session.clone(), op.view.clone(), op.deadline)
        };
        let current = self.platform.view(&context, scope).await?;
        self.current(&context).await?;
        if self
            .state
            .lock()
            .map_err(|_| "Hub setup state unavailable")?
            .operations
            .get(&key)
            .is_none_or(|op| op.session_id != id)
        {
            return Err("Hub setup was replaced by a newer attempt".into());
        }
        if let (Some(hub), Some(expected)) = (&current.hub, pairing.hub_id.as_deref()) {
            if hub.cloud.id != expected || hub.cloud.host_id != session.candidate.host_id {
                return Err("Another Hub was configured for this Space".into());
            }
            if current.configured
                && !hub.cloud.status.eq_ignore_ascii_case("setup")
                && current.operation.is_none()
            {
                if pairing.state != "COMPLETED" {
                    self.platform
                        .connected(&context, scope, expected, &session.candidate)
                        .await?;
                    self.current(&context).await?;
                    self.update(&key, id, |op| {
                        op.view.state = "COMPLETED".into();
                        op.view.phase = "COMPLETED".into();
                        op.view.failure = None;
                    });
                }
                return Ok(api::HubSetupStatusResponse {
                    scope_id: scope.into(),
                    session_id: id.into(),
                    complete: true,
                    pairing: None,
                    operation: None,
                });
            }
        }
        if Instant::now() >= deadline && pairing.state != "FAILED" {
            pairing.state = "FAILED".into();
            pairing.phase = "FAILED".into();
            pairing.failure = Some(api::HubOperationFailure {
                reason: "Hub setup was not committed before the deadline".into(),
                at: Some(now_ms()),
            });
            self.update(&key, id, |op| op.view = pairing.clone());
        }
        Ok(api::HubSetupStatusResponse {
            scope_id: scope.into(),
            session_id: id.into(),
            complete: false,
            pairing: Some(pairing),
            operation: current.operation,
        })
    }
    pub async fn cancel(
        &self,
        scope: &str,
        id: &str,
    ) -> Result<api::HubSetupCancelResponse, String> {
        let context = self.platform.account_context().await?;
        let mut state = self
            .state
            .lock()
            .map_err(|_| "Hub setup state unavailable")?;
        if !state
            .sessions
            .get(id)
            .is_some_and(|s| s.context == context && s.scope == scope)
        {
            return Err("Pairing expired or setup already started".into());
        }
        state.sessions.remove(id);
        Ok(api::HubSetupCancelResponse {
            scope_id: scope.into(),
            session_id: id.into(),
            cancelled: true,
        })
    }
    pub async fn handle(
        self: &Arc<Self>,
        scope: &str,
        target: &str,
        payload: &str,
    ) -> Result<String, String> {
        let output = match target {
            api::LOCAL_GET_TARGET => serde_json::to_value(self.view(scope).await?),
            api::LOCAL_CANDIDATES_TARGET => serde_json::to_value(self.candidates(scope).await?),
            api::LOCAL_SETUP_START_TARGET => serde_json::to_value(
                self.start(
                    scope,
                    serde_json::from_str(payload)
                        .map_err(|e| format!("Invalid Hub setup request: {e}"))?,
                )
                .await?,
            ),
            api::LOCAL_SETUP_CONFIRM_TARGET
            | api::LOCAL_SETUP_STATUS_TARGET
            | api::LOCAL_SETUP_CANCEL_TARGET => {
                let input: api::HubSetupSessionRequest = serde_json::from_str(payload)
                    .map_err(|e| format!("Invalid Hub session request: {e}"))?;
                match target {
                    api::LOCAL_SETUP_CONFIRM_TARGET => {
                        serde_json::to_value(self.confirm(scope, &input.session_id).await?)
                    }
                    api::LOCAL_SETUP_STATUS_TARGET => {
                        serde_json::to_value(self.status(scope, &input.session_id).await?)
                    }
                    _ => serde_json::to_value(self.cancel(scope, &input.session_id).await?),
                }
            }
            _ => return Err("Unknown Hub commissioning target".into()),
        }
        .map_err(|e| e.to_string())?;
        Ok(output.to_string())
    }
    async fn cloud(&self, session: &Session, path: &str, body: Value) -> Result<Value, String> {
        self.current(&session.context).await?;
        let result = tokio::time::timeout(
            READ_TIMEOUT,
            self.platform
                .cloud(&session.context, &session.scope, path, body),
        )
        .await
        .map_err(|_| "Cloud authorization timed out")??;
        self.current(&session.context).await?;
        Ok(result)
    }
    #[allow(clippy::too_many_arguments)] // Mirror Platform::lan while enforcing account and TLS checks.
    async fn lan(
        &self,
        context: &str,
        candidate: &Candidate,
        base: &str,
        pin: Option<&PublicKey>,
        path: &str,
        body: Value,
        timeout: Duration,
    ) -> Result<LanResponse, String> {
        self.current(context).await?;
        let result = tokio::time::timeout(
            timeout,
            self.platform
                .lan(context, candidate, base, pin, path, body, timeout),
        )
        .await
        .map_err(|_| "Hub request timed out")??;
        self.current(context).await?;
        if pin.is_some_and(|pin| *pin != result.peer_key) {
            return Err("Hub TLS identity changed".into());
        }
        Ok(result)
    }
    async fn challenge(
        &self,
        context: &str,
        candidate: &Candidate,
        base: &str,
        pin: Option<&PublicKey>,
        nonce: &str,
    ) -> Result<Value, String> {
        let response = self
            .lan(
                context,
                candidate,
                base,
                pin,
                "/commission/challenge",
                json!({"env":RuntimeEnv::current(),"nonce":nonce}),
                READ_TIMEOUT,
            )
            .await?;
        proof::verify(&response.body, &response.peer_key, candidate, nonce)?;
        Ok(response.body)
    }
    async fn probe(
        &self,
        context: &str,
        candidate: &Candidate,
    ) -> Result<(String, PublicKey, String), String> {
        if candidate.addresses.is_empty() || candidate.port == 0 {
            return Err("Hub has no LAN endpoint".into());
        }
        let deadline = Instant::now() + READ_TIMEOUT;
        let mut last = "Hub unavailable".to_owned();
        for (i, address) in candidate.addresses.iter().enumerate() {
            let ip = address
                .parse::<std::net::IpAddr>()
                .map_err(|_| "Hub discovery must resolve a numeric LAN address")?;
            let base = format!("https://{}", std::net::SocketAddr::new(ip, candidate.port));
            let budget = deadline.saturating_duration_since(Instant::now())
                / (candidate.addresses.len() - i) as u32;
            let nonce = uuid::Uuid::new_v4().to_string();
            match tokio::time::timeout(
                budget,
                self.challenge(context, candidate, &base, None, &nonce),
            )
            .await
            {
                Ok(Ok(body)) => {
                    return Ok((
                        base,
                        serde_json::from_value(body["hostPublicKey"].clone())
                            .map_err(|e| e.to_string())?,
                        address.clone(),
                    ))
                }
                Ok(Err(error)) => last = error,
                Err(_) => last = "Hub query timed out".into(),
            }
        }
        Err(last)
    }
}
fn text(value: &Value, key: &str) -> Result<String, String> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| format!("Hub response missing {key}"))
}
fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}
fn accepted(scope: &str, id: &str) -> api::HubSetupConfirmResponse {
    api::HubSetupConfirmResponse {
        scope_id: scope.into(),
        session_id: id.into(),
        accepted: true,
    }
}
