//! `/local/pod/*` and `/local/host/provision/*` request handling.
use std::sync::Arc;

use app_facade_api::host_provision::{self as host, ProvisionStatus};
use app_facade_api::pod::{
    self, AdapterState, BluetoothSettingsResponse, CommissionAuthorizeRequest,
    CommissionCodeRequest, CommissionSessionRequest, CommissionStartRequest, CommissionStatus,
    CommissionWifiRequest, DeviceKind, DiscoveryLease, EmptyRequest, LeaseRequest,
    RequestErrorReason, DISCOVERY_LEASE_MS, SESSION_LEASE_MS, SESSION_LIFETIME_MS,
};
use core_api::ErrorResponse;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::info;

use crate::actor::Actor;
use crate::cloud::is_loopback_url;
use crate::engine::Engine;
use crate::session::{Command, Live, Op, Phase};

/// A rejected request: the facade error code, its stable reason and a
/// developer-facing message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{code}: {message}")]
pub struct RequestError {
    pub code: &'static str,
    pub reason: Option<RequestErrorReason>,
    pub message: String,
}

impl RequestError {
    pub fn rejected(reason: RequestErrorReason, message: impl Into<String>) -> Self {
        Self {
            code: "PRECONDITION_FAIL",
            reason: Some(reason),
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST",
            reason: None,
            message: message.into(),
        }
    }
}

impl From<RequestError> for ErrorResponse {
    fn from(error: RequestError) -> Self {
        let response = ErrorResponse::new(error.code, error.message);
        match error.reason {
            Some(reason) => response.with_details(json!({ "reason": reason.as_str() })),
            None => response,
        }
    }
}

use RequestErrorReason as Reason;

/// Whether `target` is served by [`crate::Commissioning::handle`].
pub fn is_target(target: &str) -> bool {
    pod::LOCAL_TARGETS.contains(&target) || host::LOCAL_TARGETS.contains(&target)
}

fn parse<T: DeserializeOwned>(payload: &str) -> Result<T, RequestError> {
    let payload = if payload.trim().is_empty() {
        "{}"
    } else {
        payload
    };
    serde_json::from_str(payload).map_err(|error| RequestError::bad_request(error.to_string()))
}

fn to_json(value: &impl Serialize) -> Result<String, ErrorResponse> {
    serde_json::to_string(value).map_err(|error| ErrorResponse::new("INTERNAL", error.to_string()))
}

pub(crate) async fn dispatch(
    engine: &Arc<Engine>,
    target: &str,
    payload: &str,
) -> Result<String, ErrorResponse> {
    use DeviceKind::{Host, Pod};
    let value = match target {
        pod::DISCOVERY_START_TARGET => {
            let _: EmptyRequest = parse(payload)?;
            return to_json(&discovery_lease(engine, uuid::Uuid::new_v4().to_string()));
        }
        pod::DISCOVERY_RENEW_TARGET => {
            let request: LeaseRequest = parse(payload)?;
            return to_json(&discovery_lease(engine, request.lease_id));
        }
        pod::DISCOVERY_STOP_TARGET => {
            let request: LeaseRequest = parse(payload)?;
            engine
                .discovery
                .lock()
                .unwrap()
                .leases
                .remove(&request.lease_id);
            engine.reconcile();
            return to_json(&EmptyRequest {});
        }
        pod::DISCOVERY_LIST_TARGET => {
            let _: EmptyRequest = parse(payload)?;
            return to_json(&engine.discovery.lock().unwrap().snapshot());
        }
        pod::BLUETOOTH_SETTINGS_TARGET => {
            let _: EmptyRequest = parse(payload)?;
            let opened = engine.platform.radio.open_bluetooth_settings().await;
            return to_json(&BluetoothSettingsResponse { opened });
        }
        pod::COMMISSION_STATUS_TARGET => {
            let _: EmptyRequest = parse(payload)?;
            let session = engine
                .sessions
                .lock()
                .unwrap()
                .pod
                .as_ref()
                .map(Live::pod_snapshot);
            return to_json(&CommissionStatus { session });
        }
        host::PROVISION_STATUS_TARGET => {
            let _: EmptyRequest = parse(payload)?;
            let session = engine
                .sessions
                .lock()
                .unwrap()
                .host
                .as_ref()
                .map(Live::host_snapshot);
            return to_json(&ProvisionStatus { session });
        }
        pod::COMMISSION_START_TARGET => start(engine, Pod, parse(payload)?).await,
        host::PROVISION_START_TARGET => start(engine, Host, parse(payload)?).await,
        pod::COMMISSION_CODE_TARGET => code(engine, Pod, parse(payload)?).await,
        host::PROVISION_CODE_TARGET => code(engine, Host, parse(payload)?).await,
        pod::COMMISSION_WIFI_TARGET => wifi(engine, Pod, parse(payload)?).await,
        host::PROVISION_WIFI_TARGET => wifi(engine, Host, parse(payload)?).await,
        pod::COMMISSION_WIFI_SCAN_TARGET => wifi_scan(engine, Pod, parse(payload)?).await,
        host::PROVISION_WIFI_SCAN_TARGET => wifi_scan(engine, Host, parse(payload)?).await,
        pod::COMMISSION_AUTHORIZE_TARGET => authorize(engine, parse(payload)?).await,
        pod::COMMISSION_CANCEL_TARGET => cancel(engine, Pod, parse(payload)?),
        host::PROVISION_CANCEL_TARGET => cancel(engine, Host, parse(payload)?),
        pod::COMMISSION_RENEW_TARGET => renew(engine, Pod, parse(payload)?),
        host::PROVISION_RENEW_TARGET => renew(engine, Host, parse(payload)?),
        _ => {
            return Err(ErrorResponse::new(
                "UNSUPPORTED",
                format!("unsupported target: {target}"),
            ))
        }
    }?;
    Ok(value.to_string())
}

fn discovery_lease(engine: &Engine, lease_id: String) -> DiscoveryLease {
    let lease = DiscoveryLease {
        lease_id,
        expires_at_ms: engine.now_ms() + DISCOVERY_LEASE_MS,
    };
    engine
        .discovery
        .lock()
        .unwrap()
        .leases
        .insert(lease.lease_id.clone(), lease.expires_at_ms);
    engine.reconcile();
    lease
}

fn unknown(kind: DeviceKind) -> RequestError {
    RequestError::rejected(
        Reason::UnknownSession,
        format!("no {} session with this id", crate::session::noun(kind)),
    )
}

fn wrong_state(live: &Live) -> RequestError {
    RequestError::rejected(
        Reason::WrongState,
        format!("the session is {:?}", live.phase),
    )
}

fn busy() -> RequestError {
    RequestError::rejected(Reason::Busy, "the session is handling another request")
}

async fn start(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionStartRequest,
) -> Result<Value, RequestError> {
    if kind == DeviceKind::Pod {
        let account = engine.platform.cloud.account().await.ok_or_else(|| {
            RequestError::rejected(Reason::NotSignedIn, "sign in before adding a pod")
        })?;
        if account.scopes.is_empty() {
            return Err(RequestError::rejected(
                Reason::ScopeNotOffered,
                "the account has no Space",
            ));
        }
    }
    let now = engine.now_ms();
    let (snapshot, id, peripheral_id, commands, cancel) = {
        let mut sessions = engine.sessions.lock().unwrap();
        if sessions.active() {
            return Err(RequestError::rejected(
                Reason::SessionActive,
                "a pod or Host session is already running",
            ));
        }
        if sessions.radio_owned() {
            return Err(busy());
        }
        let candidate = {
            let discovery = engine.discovery.lock().unwrap();
            let candidate = discovery
                .candidates
                .get(&request.candidate_id)
                .cloned()
                .ok_or_else(|| {
                    RequestError::rejected(Reason::DeviceGone, "this device is no longer nearby")
                })?;
            if candidate.kind != kind {
                return Err(RequestError::rejected(
                    Reason::WrongKind,
                    format!("this device is a {:?}", candidate.kind),
                ));
            }
            if !candidate.commissionable {
                return Err(RequestError::rejected(
                    Reason::NotCommissionable,
                    "this device is not accepting setup now",
                ));
            }
            if discovery.adapter.state != AdapterState::Ready {
                return Err(RequestError::rejected(
                    Reason::BluetoothUnavailable,
                    "Bluetooth is not ready",
                ));
            }
            candidate
        };
        let (commands_tx, commands) = mpsc::unbounded_channel();
        let (cancel_tx, cancel) = watch::channel(false);
        let live = Live {
            kind,
            id: uuid::Uuid::new_v4().to_string(),
            revision: 1,
            phase: Phase::Connecting,
            candidate,
            expires_at_ms: now + SESSION_LIFETIME_MS,
            lease_expires_ms: now + SESSION_LEASE_MS,
            networks: Vec::new(),
            error: None,
            device: None,
            authorization: None,
            pod_id: None,
            mode: None,
            host: None,
            addresses: Vec::new(),
            claim_token: None,
            busy: false,
            scanning: false,
            released: false,
            commands: commands_tx,
            cancel: cancel_tx,
        };
        engine.emit_session(&live);
        let snapshot = live.snapshot();
        let id = live.id.clone();
        let peripheral_id = live.candidate.candidate_id.clone();
        *sessions.slot(kind) = Some(live);
        (snapshot, id, peripheral_id, commands, cancel)
    };
    info!(?kind, session = %id, "BLE setup started");
    engine.reconcile();
    engine.spawn_watchdog(kind, id.clone());
    Actor::spawn(engine.clone(), kind, id, peripheral_id, commands, cancel);
    Ok(snapshot)
}

/// Runs `check` on the current session under the lock. When it yields an op,
/// the op is queued for the session task and the reply waits for it.
async fn request(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    id: &str,
    check: impl FnOnce(&mut Live) -> Result<Option<Op>, RequestError>,
) -> Result<Value, RequestError> {
    let done = {
        let mut sessions = engine.sessions.lock().unwrap();
        let live = sessions.current(kind, id).ok_or_else(|| unknown(kind))?;
        let before = live.revision;
        let op = check(live)?;
        if live.revision != before {
            engine.emit_session(live);
        }
        op.and_then(|op| {
            let (done, wait) = oneshot::channel();
            live.commands.send(Command { op, done }).ok().map(|_| wait)
        })
    };
    if let Some(done) = done {
        let _ = done.await;
    }
    let mut sessions = engine.sessions.lock().unwrap();
    let live = sessions.current(kind, id).ok_or_else(|| unknown(kind))?;
    Ok(live.snapshot())
}

async fn code(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionCodeRequest,
) -> Result<Value, RequestError> {
    let code = request.code.trim().to_string();
    self::request(engine, kind, &request.session_id, |live| {
        if !crate::device::valid_code(&code) {
            return Err(RequestError::rejected(
                Reason::InvalidCode,
                "the code must be 6 digits",
            ));
        }
        if live.phase != Phase::AwaitingCode {
            return Err(wrong_state(live));
        }
        if live.busy {
            return Err(busy());
        }
        live.busy = true;
        live.set(Phase::Securing, None);
        live.revision += 1;
        Ok(Some(Op::Code(code)))
    })
    .await
}

async fn wifi(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionWifiRequest,
) -> Result<Value, RequestError> {
    let password = request.password.unwrap_or_default();
    let ssid = request.ssid;
    self::request(engine, kind, &request.session_id, |live| {
        if live.phase != Phase::AwaitingWifi {
            return Err(wrong_state(live));
        }
        if !crate::device::valid_wifi(&ssid, &password) {
            return Err(RequestError::rejected(
                Reason::InvalidWifi,
                "the network name must be 1 to 32 bytes and the password empty, 8 to 63 characters or 64 hex digits",
            ));
        }
        if live.busy {
            return Err(busy());
        }
        live.busy = true;
        Ok(Some(Op::Wifi { ssid, password }))
    })
    .await
}

async fn wifi_scan(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionSessionRequest,
) -> Result<Value, RequestError> {
    self::request(engine, kind, &request.session_id, |live| {
        if live.phase != Phase::AwaitingWifi {
            return Err(wrong_state(live));
        }
        if live.busy {
            return Err(busy());
        }
        let join = live.scanning;
        live.scanning = true;
        Ok(Some(Op::Scan { join }))
    })
    .await
}

async fn authorize(
    engine: &Arc<Engine>,
    request: CommissionAuthorizeRequest,
) -> Result<Value, RequestError> {
    let endpoints = engine.platform.cloud.cloud_endpoints().await;
    let scope_id = request.scope_id;
    self::request(engine, DeviceKind::Pod, &request.session_id, |live| {
        if live.phase != Phase::AwaitingAuthorization {
            return Err(wrong_state(live));
        }
        if live.busy {
            return Err(busy());
        }
        let offered = live
            .authorization
            .as_ref()
            .is_some_and(|prompt| prompt.scopes.iter().any(|scope| scope.scope_id == scope_id));
        if !offered {
            return Err(RequestError::rejected(
                Reason::ScopeNotOffered,
                "choose one of the offered Spaces",
            ));
        }
        if is_loopback_url(&endpoints.api) || is_loopback_url(&endpoints.ws) {
            return Err(RequestError::rejected(
                Reason::CloudUnreachableForDevice,
                "The pod can't reach this MeowLink cloud address. Use an address on your network.",
            ));
        }
        live.busy = true;
        live.set(Phase::Issuing, None);
        live.revision += 1;
        Ok(Some(Op::Authorize {
            scope_id,
            endpoints,
        }))
    })
    .await
}

fn cancel(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionSessionRequest,
) -> Result<Value, RequestError> {
    engine.terminate(kind, &request.session_id, Phase::Cancelled, None);
    let mut sessions = engine.sessions.lock().unwrap();
    let live = sessions
        .current(kind, &request.session_id)
        .ok_or_else(|| unknown(kind))?;
    Ok(live.snapshot())
}

fn renew(
    engine: &Arc<Engine>,
    kind: DeviceKind,
    request: CommissionSessionRequest,
) -> Result<Value, RequestError> {
    let now = engine.now_ms();
    let mut sessions = engine.sessions.lock().unwrap();
    let live = sessions
        .current(kind, &request.session_id)
        .ok_or_else(|| unknown(kind))?;
    if !live.phase.is_terminal() {
        live.lease_expires_ms = now + SESSION_LEASE_MS;
    }
    Ok(live.snapshot())
}
