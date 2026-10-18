use super::{Authority, ManagementActor};
use axum::{
    body::{to_bytes, Body},
    extract::{DefaultBodyLimit, Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use core_api::host::{
    access::*,
    management::{HostRuntimeAction, HostRuntimeRequest},
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::Semaphore;

/// Optional device-local admission (for example a BLE provisioning token).
/// Implementations run on a blocking worker, never an HTTP executor thread.
pub trait ClaimAdmission: Send + Sync + 'static {
    fn check(&self, request: &ClaimRequest) -> Result<(), ClaimAdmissionError>;
    fn committed(&self);
}
pub enum ClaimAdmissionError {
    Required,
    Unavailable,
}
pub struct DirectClaim;
impl ClaimAdmission for DirectClaim {
    fn check(&self, _: &ClaimRequest) -> Result<(), ClaimAdmissionError> {
        Ok(())
    }
    fn committed(&self) {}
}
#[derive(Clone)]
struct BootstrapState {
    authority: Arc<Authority>,
    host_name: String,
    admission: Arc<dyn ClaimAdmission>,
}

const BOOTSTRAP_BURST: f64 = 32.0;
const BOOTSTRAP_PER_SECOND: f64 = 8.0;
const BOOTSTRAP_CONCURRENCY: usize = 4;

/// A fixed-size admission gate, independent of anonymous identities or addresses.
struct BootstrapAdmission {
    tokens: Mutex<(f64, Instant)>,
    concurrent: Semaphore,
}
impl BootstrapAdmission {
    fn new() -> Self {
        Self {
            tokens: Mutex::new((BOOTSTRAP_BURST, Instant::now())),
            concurrent: Semaphore::new(BOOTSTRAP_CONCURRENCY),
        }
    }
    fn take_token(&self) -> bool {
        let mut bucket = self.tokens.lock().unwrap();
        let now = Instant::now();
        bucket.0 = (bucket.0 + now.duration_since(bucket.1).as_secs_f64() * BOOTSTRAP_PER_SECOND)
            .min(BOOTSTRAP_BURST);
        bucket.1 = now;
        if bucket.0 < 1.0 {
            return false;
        }
        bucket.0 -= 1.0;
        true
    }
}
async fn admit_bootstrap(
    State(admission): State<Arc<BootstrapAdmission>>,
    request: Request,
    next: Next,
) -> Response {
    // No queue: reject before reading JSON, verifying proofs or calling the cloud.
    let Ok(_permit) = admission.concurrent.try_acquire() else {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    };
    if !admission.take_token() {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    // Bound slow-body occupancy as well as body size and cryptographic concurrency.
    match tokio::time::timeout(std::time::Duration::from_secs(15), next.run(request)).await {
        Ok(response) => response,
        Err(_) => StatusCode::REQUEST_TIMEOUT.into_response(),
    }
}

pub fn public_routes(
    authority: Arc<Authority>,
    host_name: String,
    admission: Arc<dyn ClaimAdmission>,
) -> Router {
    Router::new()
        .route("/v1/auth/status", get(status))
        .route("/v1/auth/challenge", post(challenge))
        .route("/v1/auth/administrators", post(application))
        .route("/v1/auth/claim", post(claim))
        .with_state(BootstrapState {
            authority,
            host_name,
            admission,
        })
        .layer(DefaultBodyLimit::max(32 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            Arc::new(BootstrapAdmission::new()),
            admit_bootstrap,
        ))
}
async fn application(
    State(state): State<BootstrapState>,
    Json(request): Json<HostRuntimeRequest<serde_json::Value>>,
) -> Response {
    let authority = state.authority;
    if request.host_id != authority.host_id {
        return reject(StatusCode::BAD_REQUEST, "HOST_IDENTITY_MISMATCH");
    }
    let (action, proof) = match request.action {
        HostRuntimeAction::AdministratorRequest { proof } => ("administratorRequest", proof),
        HostRuntimeAction::AdministratorRequestStatus { proof } => {
            ("administratorRequestStatus", proof)
        }
        HostRuntimeAction::AdministratorRequestCancel { proof } => {
            ("administratorRequestCancel", proof)
        }
        _ => return reject(StatusCode::BAD_REQUEST, "HOST_INVALID_APPLICANT_OPERATION"),
    };
    match authority.administrator_application(action, proof).await {
        Ok(data) => Json(core_api::host::management::HostRuntimeResponse {
            host_id: authority.host_id.clone(),
            data,
        })
        .into_response(),
        Err(_) => reject(StatusCode::FORBIDDEN, "HOST_ADMINISTRATOR_REQUEST_REJECTED"),
    }
}
async fn status(State(state): State<BootstrapState>) -> Response {
    match state.authority.status() {
        Ok(status) => Json(status).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}
async fn challenge(
    State(state): State<BootstrapState>,
    Json(request): Json<ChallengeRequest>,
) -> Response {
    bootstrap_response(state.authority.challenge(state.host_name, request).await)
}
async fn claim(State(state): State<BootstrapState>, Json(request): Json<ClaimRequest>) -> Response {
    let gate = state.admission.clone();
    let admission_request = request.clone();
    match tokio::task::spawn_blocking(move || gate.check(&admission_request)).await {
        Ok(Ok(())) => {}
        Ok(Err(ClaimAdmissionError::Required)) => {
            return reject(
                StatusCode::FORBIDDEN,
                core_api::host::auth::CLAIM_TOKEN_REQUIRED,
            )
        }
        _ => return StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
    match state.authority.complete_claim(request).await {
        Ok(receipt) => {
            let gate = state.admission;
            let _ = tokio::task::spawn_blocking(move || gate.committed()).await;
            Json(serde_json::json!({"proof":receipt})).into_response()
        }
        Err(error) => {
            tracing::warn!(error=%error, "Host claim rejected");
            reject(StatusCode::FORBIDDEN, "HOST_CLAIM_REJECTED")
        }
    }
}
fn bootstrap_response(result: anyhow::Result<String>) -> Response {
    match result {
        Ok(proof) => Json(serde_json::json!({"proof":proof})).into_response(),
        Err(error) => {
            tracing::warn!(error=%error, "Host authorization rejected");
            reject(StatusCode::FORBIDDEN, "HOST_INVALID_PROOF")
        }
    }
}
fn reject(status: StatusCode, code: &str) -> Response {
    (
        status,
        [(header::CACHE_CONTROL, "no-store")],
        Json(serde_json::json!({"code":code})),
    )
        .into_response()
}
fn permission(
    method: &Method,
    path: &str,
    body: &[u8],
    host_id: &str,
) -> anyhow::Result<Permission> {
    if method == Method::GET
        && matches!(
            path,
            "/info" | "/metrics" | "/v1/services" | "/v1/permissions"
        )
    {
        return Ok(Permission::Read);
    }
    if method == Method::POST {
        if path == "/v1/runtime" {
            let request: HostRuntimeRequest<serde_json::Value> = serde_json::from_slice(body)?;
            anyhow::ensure!(request.host_id == host_id, "runtime targets another Host");
            return Ok(match request.action {
                HostRuntimeAction::Administrators
                | HostRuntimeAction::AdministratorDecide { .. }
                | HostRuntimeAction::AdministratorRemove { .. } => Permission::ManageAdministrators,
                HostRuntimeAction::AdministratorRequest { .. }
                | HostRuntimeAction::AdministratorRequestStatus { .. }
                | HostRuntimeAction::AdministratorRequestCancel { .. } => {
                    anyhow::bail!("Applicant requests use the applicant endpoint")
                }
                HostRuntimeAction::Inspect
                | HostRuntimeAction::Metrics
                | HostRuntimeAction::Operation { .. } => Permission::Read,
                HostRuntimeAction::ConfigureService { .. }
                | HostRuntimeAction::Plan { .. }
                | HostRuntimeAction::Start { .. }
                | HostRuntimeAction::RestartOperation { .. } => Permission::ManageRuntime,
            });
        }
        if path == "/v1/factory-reset" {
            return Ok(Permission::Reset);
        }
    }
    anyhow::bail!("Unsupported Host management operation")
}
pub async fn authenticate(
    State(authority): State<Arc<Authority>>,
    request: Request,
    next: Next,
) -> Response {
    let (mut parts, body) = request.into_parts();
    if parts.headers.get_all(header::AUTHORIZATION).iter().count() != 1 {
        return reject(StatusCode::UNAUTHORIZED, "HOST_TOKEN_REQUIRED");
    }
    let token = parts
        .headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.split_once(' '))
        .filter(|(kind, value)| {
            kind.eq_ignore_ascii_case("Bearer") && !value.is_empty() && value.len() <= 16384
        })
        .map(|(_, value)| value.to_owned());
    let Some(token) = token else {
        return reject(StatusCode::UNAUTHORIZED, "HOST_TOKEN_REQUIRED");
    };
    let body = match tokio::time::timeout(
        std::time::Duration::from_secs(15),
        to_bytes(body, 1024 * 1024),
    )
    .await
    {
        Ok(Ok(body)) => body,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let action = match permission(&parts.method, parts.uri.path(), &body, &authority.host_id) {
        Ok(action) => action,
        Err(_) => return StatusCode::BAD_REQUEST.into_response(),
    };
    match authority.authenticate(&token, action).await {
        Ok(user) => {
            parts.extensions.insert(ManagementActor::CloudUser(user));
        }
        Err(error) => {
            tracing::debug!(error=%error, "Host management authorization rejected");
            return reject(StatusCode::FORBIDDEN, "HOST_AUTHORIZATION_DENIED");
        }
    }
    let mut response = next.run(Request::from_parts(parts, Body::from(body))).await;
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_runtime_actions_have_an_explicit_permission_and_exact_host_target() {
        for (action, expected) in [
            (
                serde_json::json!({"action":"administrators"}),
                Permission::ManageAdministrators,
            ),
            (
                serde_json::json!({"action":"administratorDecide","requestId":"request","approve":true}),
                Permission::ManageAdministrators,
            ),
            (
                serde_json::json!({"action":"administratorRemove","userId":"bob"}),
                Permission::ManageAdministrators,
            ),
            (
                serde_json::json!({"action":"configureService", "serviceId":"core", "enabled":true}),
                Permission::ManageRuntime,
            ),
            (serde_json::json!({"action":"inspect"}), Permission::Read),
            (serde_json::json!({"action":"metrics"}), Permission::Read),
            (
                serde_json::json!({"action":"operation","operationId":"op"}),
                Permission::Read,
            ),
            (
                serde_json::json!({"action":"plan","components":[],"all":true}),
                Permission::ManageRuntime,
            ),
            (
                serde_json::json!({"action":"start","planId":"plan"}),
                Permission::ManageRuntime,
            ),
            (
                serde_json::json!({"action":"restartOperation","operationId":"op","component":"core"}),
                Permission::ManageRuntime,
            ),
        ] {
            let mut body = action;
            body["hostId"] = "host".into();
            let bytes = serde_json::to_vec(&body).unwrap();
            assert_eq!(
                permission(&Method::POST, "/v1/runtime", &bytes, "host").unwrap(),
                expected
            );
            assert!(permission(&Method::POST, "/v1/runtime", &bytes, "other").is_err());
        }
        assert_eq!(
            permission(&Method::POST, "/v1/factory-reset", b"{}", "host").unwrap(),
            Permission::Reset
        );
        assert!(permission(&Method::POST, "/v1/auth/unenroll", b"{}", "host").is_err());
    }
}
