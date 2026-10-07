//! The task that owns one session's link and runs its device and cloud steps
//! in order.
use std::future::Future;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use app_facade_api::host_provision::{is_claim_token, HostFinishResponse, HOST_INFO_ENDPOINT};
use app_facade_api::pod::{
    AuthorizationPrompt, CommissionError, CommissionErrorCode, CredentialIssueRequest,
    CredentialStatus, DeviceKind, PodInfo, SessionMode, ACTIVATION_GRACE_MS, CODE_ATTEMPTS,
    CREDENTIAL_STATUS_POLL_MS, NOT_ACTIVATED_DETAIL, POD_CREDENTIAL_ENDPOINT, POD_INFO_ENDPOINT,
    SRP_USERNAME,
};
use protocomm::{ProtocommError, WifiFailure, WifiStatus};
use serde_json::{json, Value};
use tokio::sync::{mpsc, oneshot, watch};
use tracing::{info, warn};

use crate::cloud;
use crate::device::{self, OpenError, Opened};
use crate::engine::Engine;
use crate::platform::CloudEndpoints;
use crate::session::{disconnected, error, noun, Command, Op, Phase};

const TICK: Duration = Duration::from_secs(1);
const POD_JOIN_POLLS: u32 = 60;
/// The Host waits up to 45 s for its network manager before reporting a failure.
const HOST_JOIN_POLLS: u32 = 75;
/// How long a pod that kept its network may stay `connecting` before the
/// commissioner offers new Wi-Fi credentials.
const KEPT_WIFI_POLLS: u32 = 30;
const FINISH_ATTEMPTS: u32 = 3;
/// How long a pod in a Wi-Fi change window may take to save the new network
/// after joining it.
const REPROVISION_POLLS: u32 = 30;

struct Credential {
    pod_id: String,
    activate_before: i64,
    context: String,
    /// `completed`, or Lion reported it absent: nothing to revoke.
    settled: bool,
}

pub(crate) struct Actor {
    engine: Arc<Engine>,
    kind: DeviceKind,
    id: String,
    peripheral_id: String,
    cancel: watch::Receiver<bool>,
    device: Option<Opened>,
    rejected: u32,
    join_failed: bool,
    pod_info: Option<PodInfo>,
    credential: Option<Credential>,
    reply: Option<oneshot::Sender<()>>,
}

async fn cancelled(cancel: &mut watch::Receiver<bool>) {
    if cancel.wait_for(|cancelled| *cancelled).await.is_err() {
        std::future::pending::<()>().await;
    }
}

fn wifi_error(kind: DeviceKind, failure: WifiFailure) -> CommissionError {
    let noun = noun(kind);
    match failure {
        WifiFailure::AuthError => error(
            CommissionErrorCode::WifiAuthFailed,
            "The Wi-Fi password was rejected.",
            None,
        ),
        WifiFailure::NetworkNotFound => error(
            CommissionErrorCode::WifiNotFound,
            &format!("The {noun} could not find this Wi-Fi network."),
            None,
        ),
        WifiFailure::Other => error(
            CommissionErrorCode::WifiFailed,
            &format!("The {noun} could not join this Wi-Fi network."),
            None,
        ),
    }
}

impl Actor {
    pub fn spawn(
        engine: Arc<Engine>,
        kind: DeviceKind,
        id: String,
        peripheral_id: String,
        commands: mpsc::UnboundedReceiver<Command>,
        cancel: watch::Receiver<bool>,
    ) {
        let actor = Self {
            engine,
            kind,
            id,
            peripheral_id,
            cancel,
            device: None,
            rejected: 0,
            join_failed: false,
            pod_info: None,
            credential: None,
            reply: None,
        };
        tokio::spawn(actor.run(commands));
    }

    async fn run(mut self, mut commands: mpsc::UnboundedReceiver<Command>) {
        self.engine.pause_scanning().await;
        self.connect().await;
        loop {
            if self.over() {
                break;
            }
            let mut cancel = self.cancel.clone();
            tokio::select! {
                command = commands.recv() => {
                    let Some(command) = command else { break };
                    self.reply = Some(command.done);
                    self.execute(command.op).await;
                    self.ack();
                }
                _ = tokio::time::sleep(TICK) => self.check_link().await,
                _ = cancelled(&mut cancel) => {}
            }
        }
        commands.close();
        self.cleanup().await;
    }

    /// Answers the request being executed; later steps run on without it.
    fn ack(&mut self) {
        if let Some(reply) = self.reply.take() {
            self.engine
                .mutate(self.kind, &self.id, |live| live.busy = false);
            let _ = reply.send(());
        }
    }

    fn over(&self) -> bool {
        self.engine
            .phase(self.kind, &self.id)
            .is_none_or(Phase::is_terminal)
    }

    async fn race<T>(&self, work: impl Future<Output = T>) -> Option<T> {
        let mut cancel = self.cancel.clone();
        tokio::select! {
            biased;
            _ = cancelled(&mut cancel) => None,
            out = work => Some(out),
        }
    }

    async fn sleep(&self, duration: Duration) -> Option<()> {
        self.race(tokio::time::sleep(duration)).await
    }

    fn update(&self, change: impl FnOnce(&mut crate::session::Live)) -> bool {
        self.engine.update(self.kind, &self.id, change)
    }

    fn enter(&self, phase: Phase, failure: Option<CommissionError>) -> bool {
        self.update(|live| live.set(phase, failure))
    }

    fn fail(&self, failure: CommissionError) {
        self.enter(Phase::Failed, Some(failure));
    }

    fn fail_disconnected(&self, detail: impl ToString) {
        self.fail(disconnected(self.kind, detail));
    }

    fn client(&self) -> Option<Arc<device::Client>> {
        self.device.as_ref().map(|device| device.client.clone())
    }

    async fn open(&mut self) -> Option<bool> {
        let opened = self
            .race(device::open(
                &self.engine.platform.central,
                &self.peripheral_id,
                self.kind,
            ))
            .await?;
        match opened {
            Ok(opened) => {
                let locked = opened.locked;
                self.device = Some(opened);
                Some(locked)
            }
            Err(OpenError::Unavailable(detail)) => {
                self.fail(error(
                    CommissionErrorCode::BluetoothUnavailable,
                    "Bluetooth is not available.",
                    Some(detail),
                ));
                None
            }
            Err(OpenError::Link(detail)) => {
                self.fail_disconnected(detail);
                None
            }
            Err(OpenError::Incompatible(raw)) => {
                self.fail(error(
                    CommissionErrorCode::DeviceBusy,
                    match self.kind {
                        DeviceKind::Pod => "This device cannot be added as a pod.",
                        DeviceKind::Host => "This device cannot be set up as a MeowLink Host.",
                    },
                    Some(raw),
                ));
                None
            }
        }
    }

    fn code_locked(&self) {
        self.fail(error(
            CommissionErrorCode::CodeLocked,
            &format!(
                "Too many wrong codes. Wait for the {} to show a new code, then start again.",
                noun(self.kind)
            ),
            None,
        ));
    }

    async fn connect(&mut self) {
        match self.open().await {
            Some(true) => self.code_locked(),
            Some(false) => {
                self.enter(Phase::AwaitingCode, None);
            }
            None => {}
        }
    }

    async fn check_link(&mut self) {
        let waiting = self
            .engine
            .phase(self.kind, &self.id)
            .is_some_and(Phase::waits_for_user);
        let Some(device) = self.device.as_ref().filter(|_| waiting) else {
            return;
        };
        if !device.link.is_connected().await {
            self.fail_disconnected(format!("the {} closed the connection", noun(self.kind)));
        }
    }

    async fn execute(&mut self, op: Op) {
        match op {
            Op::Code(code) => self.code(code).await,
            Op::Wifi { ssid, password } => self.wifi(ssid, password).await,
            Op::Scan { join: true } => {}
            Op::Scan { join: false } => self.scan().await,
            Op::Authorize {
                scope_id,
                endpoints,
            } => self.authorize(scope_id, endpoints).await,
        }
    }

    async fn code(&mut self, code: String) {
        let Some(device) = self.device.as_ref() else {
            return self.fail_disconnected("no link");
        };
        let (client, patch) = (device.client.clone(), device.patch_version);
        let Some(result) = self
            .race(client.establish_sec2(SRP_USERNAME, &code, patch))
            .await
        else {
            return;
        };
        match result {
            Ok(()) => {
                if self.enter(Phase::ReadingInfo, None) {
                    self.ack();
                    self.read_info().await;
                }
            }
            Err(ProtocommError::CodeRejected) => self.code_rejected(true).await,
            Err(ProtocommError::HandshakeRefused) => self.code_rejected(false).await,
            Err(failure) => self.fail_disconnected(failure),
        }
    }

    async fn code_rejected(&mut self, counted: bool) {
        if counted {
            self.rejected += 1;
        }
        if self.kind == DeviceKind::Pod && self.rejected >= CODE_ATTEMPTS {
            return self.code_locked();
        }
        let Some(client) = self.client() else {
            return self.fail_disconnected("no link");
        };
        let locked = match self.race(device::is_locked(&client)).await {
            None => return,
            Some(Some(locked)) => locked,
            Some(None) => {
                if let Some(device) = self.device.take() {
                    device.link.disconnect().await;
                }
                match self.open().await {
                    Some(locked) => locked,
                    None => return,
                }
            }
        };
        if locked {
            return self.code_locked();
        }
        let message = if self.kind == DeviceKind::Host
            && self.rejected > 0
            && self.rejected.is_multiple_of(CODE_ATTEMPTS)
        {
            "The code did not match. The Host now shows a new code; enter that one.".to_string()
        } else {
            format!(
                "The code did not match. Check the code on the {} and try again.",
                noun(self.kind)
            )
        };
        self.enter(
            Phase::AwaitingCode,
            Some(error(CommissionErrorCode::CodeRejected, &message, None)),
        );
    }

    /// After a failed call, whether the secure session survived it.
    async fn survived(&self, client: &device::Client, failure: &ProtocommError) -> bool {
        !matches!(
            failure,
            ProtocommError::Disconnected(_) | ProtocommError::NotSecured
        ) && client.is_secured().await
    }

    async fn read_info(&mut self) {
        let Some(client) = self.client() else {
            return self.fail_disconnected("no link");
        };
        let request = match self.kind {
            DeviceKind::Pod => (POD_INFO_ENDPOINT, json!({})),
            DeviceKind::Host => (HOST_INFO_ENDPOINT, json!({ "op": "info" })),
        };
        let Some(info) = self.race(client.call_json(request.0, &request.1)).await else {
            return;
        };
        let info = match info {
            Ok(info) => info,
            Err(failure) => return self.fail_disconnected(failure),
        };
        let incomplete = || {
            error(
                CommissionErrorCode::DeviceBusy,
                &format!("The {} returned incomplete information.", noun(self.kind)),
                Some(info.to_string()),
            )
        };
        match self.kind {
            DeviceKind::Pod => {
                let Some((pod_info, device)) = device::pod_info(&info) else {
                    return self.fail(incomplete());
                };
                let reprovision = pod_info.reprovision();
                let kept_wifi = device.wifi_configured && !reprovision;
                self.pod_info = Some(pod_info);
                if !self.update(|live| {
                    live.device = Some(device);
                    live.mode = reprovision.then_some(SessionMode::Reprovision);
                }) {
                    return;
                }
                if kept_wifi {
                    self.confirm_kept_wifi(client).await;
                } else {
                    self.offer_wifi(None).await;
                }
            }
            DeviceKind::Host => {
                let Some(host) = device::host_info(&info) else {
                    return self.fail(incomplete());
                };
                if self.update(|live| live.host = Some(host)) {
                    self.offer_wifi(None).await;
                }
            }
        }
    }

    async fn confirm_kept_wifi(&mut self, client: Arc<device::Client>) {
        for _ in 0..KEPT_WIFI_POLLS {
            let Some(status) = self.race(client.wifi_status()).await else {
                return;
            };
            match status {
                Ok(WifiStatus::Connected { .. }) => return self.prompt_authorization().await,
                Ok(WifiStatus::Connecting) => {}
                Ok(WifiStatus::Failed(failure)) => {
                    self.join_failed = true;
                    return self.offer_wifi(Some(wifi_error(self.kind, failure))).await;
                }
                Err(failure) => {
                    if !self.survived(&client, &failure).await {
                        return self.fail_disconnected(failure);
                    }
                    self.join_failed = true;
                    return self.offer_wifi(None).await;
                }
            }
            if self.sleep(TICK).await.is_none() {
                return;
            }
        }
        self.join_failed = true;
        self.offer_wifi(Some(error(
            CommissionErrorCode::WifiFailed,
            "The pod could not reconnect to its Wi-Fi network.",
            None,
        )))
        .await;
    }

    async fn offer_wifi(&mut self, failure: Option<CommissionError>) {
        if self.update(|live| {
            live.set(Phase::AwaitingWifi, failure);
            live.scanning = true;
        }) {
            self.scan_now().await;
        }
    }

    async fn scan(&mut self) {
        let ready = self
            .engine
            .mutate(self.kind, &self.id, |live| {
                live.scanning = live.phase == Phase::AwaitingWifi;
                live.scanning
            })
            .unwrap_or(false);
        if ready {
            self.scan_now().await;
        }
    }

    async fn scan_now(&mut self) {
        let Some(client) = self.client() else {
            return self.fail_disconnected("no link");
        };
        let result = self.race(client.wifi_scan()).await;
        self.engine
            .mutate(self.kind, &self.id, |live| live.scanning = false);
        match result {
            None => {}
            Some(Ok(entries)) => {
                let found = device::networks(entries);
                self.update(|live| live.networks = found);
            }
            Some(Err(failure)) => {
                if !self.survived(&client, &failure).await {
                    self.fail_disconnected(failure);
                } else {
                    warn!(%failure, "Wi-Fi scan failed");
                }
            }
        }
    }

    async fn wifi(&mut self, ssid: String, password: String) {
        if self.engine.phase(self.kind, &self.id) != Some(Phase::AwaitingWifi) {
            return;
        }
        let Some(client) = self.client() else {
            return self.fail_disconnected("no link");
        };
        let join_failed = self.join_failed;
        let applied = self
            .race(async {
                if join_failed {
                    client.wifi_reset().await?;
                }
                client.wifi_set_config(&ssid, &password).await?;
                client.wifi_apply().await
            })
            .await;
        match applied {
            None => {}
            Some(Ok(())) => {
                self.join_failed = false;
                if self.enter(Phase::JoiningWifi, None) {
                    self.ack();
                    self.poll_join(client).await;
                }
            }
            Some(Err(failure)) => {
                if !self.survived(&client, &failure).await {
                    return self.fail_disconnected(failure);
                }
                self.join_failed = true;
                self.update(|live| {
                    live.error = Some(error(
                        CommissionErrorCode::WifiFailed,
                        &format!("The {} could not use this Wi-Fi network.", noun(self.kind)),
                        Some(failure.to_string()),
                    ))
                });
            }
        }
    }

    async fn poll_join(&mut self, client: Arc<device::Client>) {
        let polls = match self.kind {
            DeviceKind::Pod => POD_JOIN_POLLS,
            DeviceKind::Host => HOST_JOIN_POLLS,
        };
        for _ in 0..polls {
            if self.sleep(TICK).await.is_none() {
                return;
            }
            let Some(status) = self.race(client.wifi_status()).await else {
                return;
            };
            match status {
                Ok(WifiStatus::Connecting) => {}
                Ok(WifiStatus::Connected { ip4_addr }) => {
                    return match self.kind {
                        DeviceKind::Pod if self.reprovision() => {
                            self.confirm_new_network(client).await
                        }
                        DeviceKind::Pod => self.prompt_authorization().await,
                        DeviceKind::Host => self.complete_host(client, ip4_addr).await,
                    };
                }
                Ok(WifiStatus::Failed(failure)) => {
                    self.join_failed = true;
                    self.enter(Phase::AwaitingWifi, Some(wifi_error(self.kind, failure)));
                    return;
                }
                Err(failure) => return self.fail_disconnected(failure),
            }
        }
        self.join_failed = true;
        self.enter(
            Phase::AwaitingWifi,
            Some(error(
                CommissionErrorCode::WifiFailed,
                &format!("The {} took too long to join Wi-Fi.", noun(self.kind)),
                None,
            )),
        );
    }

    fn reprovision(&self) -> bool {
        self.pod_info.as_ref().is_some_and(PodInfo::reprovision)
    }

    /// A pod in a Wi-Fi change window saves the joined network, then reports
    /// `commissioned`; it keeps its credential.
    async fn confirm_new_network(&mut self, client: Arc<device::Client>) {
        for _ in 0..REPROVISION_POLLS {
            let Some(status) = self
                .race(client.call_json(POD_CREDENTIAL_ENDPOINT, &json!({ "op": "status" })))
                .await
            else {
                return;
            };
            match status {
                Ok(status) => match status.get("state").and_then(Value::as_str) {
                    Some("commissioned") => {
                        if self.enter(Phase::Completed, None) {
                            self.finish_pod(&client).await;
                        }
                        return;
                    }
                    Some("failed") => {
                        let code = status
                            .pointer("/error/code")
                            .and_then(Value::as_str)
                            .map(str::to_string);
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "The pod could not save the new Wi-Fi network.",
                            code,
                        ));
                    }
                    _ => {}
                },
                Err(failure) => {
                    if !self.survived(&client, &failure).await {
                        return self.fail_disconnected(failure);
                    }
                }
            }
            if self.sleep(TICK).await.is_none() {
                return;
            }
        }
        self.fail(error(
            CommissionErrorCode::WifiFailed,
            "The pod did not confirm the new Wi-Fi network.",
            None,
        ));
    }

    async fn finish_pod(&self, client: &device::Client) {
        if let Err(failure) = client
            .call_json(POD_CREDENTIAL_ENDPOINT, &json!({ "op": "finish" }))
            .await
        {
            info!(%failure, "pod finish was not delivered");
        }
    }

    async fn complete_host(&mut self, client: Arc<device::Client>, ip4_addr: String) {
        let mut claim_token = None;
        for attempt in 0..FINISH_ATTEMPTS {
            if attempt > 0 && self.sleep(TICK).await.is_none() {
                return;
            }
            let Some(result) = self
                .race(client.call_json(HOST_INFO_ENDPOINT, &json!({ "op": "finish" })))
                .await
            else {
                return;
            };
            match result {
                Ok(response) => {
                    claim_token = serde_json::from_value::<HostFinishResponse>(response)
                        .ok()
                        .and_then(|response| response.claim_token)
                        .filter(|token| is_claim_token(token));
                    break;
                }
                Err(failure) => {
                    warn!(%failure, "Host setup finish was not delivered");
                    if !self.survived(&client, &failure).await {
                        break;
                    }
                }
            }
        }
        let addresses: Vec<String> = Some(ip4_addr)
            .filter(|address| address.parse::<Ipv4Addr>().is_ok())
            .into_iter()
            .collect();
        self.update(|live| {
            live.set(Phase::Completed, None);
            live.addresses = addresses;
            live.claim_token = claim_token;
        });
    }

    async fn prompt_authorization(&mut self) {
        let Some(account) = self.race(self.engine.platform.cloud.account()).await else {
            return;
        };
        let Some(account) = account else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "Sign in to MeowLink before adding a pod.",
                None,
            ));
        };
        let Some(first) = account.scopes.first() else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "Join a Space in MeowLink before adding a pod.",
                None,
            ));
        };
        let default_scope_id = account
            .active_scope_id
            .clone()
            .filter(|active| account.scopes.iter().any(|scope| &scope.scope_id == active))
            .unwrap_or_else(|| first.scope_id.clone());
        let prompt = AuthorizationPrompt {
            user_id: account.user_id,
            user_name: account.user_name,
            scopes: account.scopes,
            default_scope_id,
        };
        self.update(|live| {
            live.set(Phase::AwaitingAuthorization, None);
            live.authorization = Some(prompt);
        });
    }

    async fn authorize(&mut self, scope_id: String, endpoints: CloudEndpoints) {
        let cloud = self.engine.platform.cloud.clone();
        let issue_failed = |detail: Option<String>| {
            error(
                CommissionErrorCode::IssueFailed,
                "MeowLink could not create the pod credential.",
                detail,
            )
        };
        let Some(account) = self.race(cloud.account()).await else {
            return;
        };
        let prompted = self
            .engine
            .mutate(self.kind, &self.id, |live| {
                live.authorization
                    .as_ref()
                    .map(|prompt| prompt.user_id.clone())
            })
            .flatten();
        let Some(account) = account.filter(|account| Some(&account.user_id) == prompted.as_ref())
        else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "The MeowLink account changed. Start again.",
                None,
            ));
        };
        let (Some(info), Some(device)) = (self.pod_info.clone(), self.device.as_ref()) else {
            return self.fail_disconnected("no link");
        };
        let client = device.client.clone();
        let name = self
            .engine
            .mutate(self.kind, &self.id, |live| live.candidate.name.clone())
            .unwrap_or_default();
        let request = CredentialIssueRequest {
            device_id: info.device_id,
            model: info.model,
            platform: Some(info.platform)
                .filter(|platform| !platform.is_empty())
                .unwrap_or_else(|| "unknown".into()),
            firmware_version: info.firmware_version,
            name,
            identity: info.identity,
            client_id: cloud.client_id().await,
        };
        let issued = match cloud::issue(&cloud, &account.context, &scope_id, &request).await {
            Ok(issued) => issued,
            Err(failure) => return self.fail(issue_failed(Some(failure.to_string()))),
        };
        self.credential = Some(Credential {
            pod_id: issued.pod_id.clone(),
            activate_before: issued.activate_before,
            context: account.context,
            settled: false,
        });
        let deadline = issued.activate_before + ACTIVATION_GRACE_MS;
        if !self.update(|live| {
            live.set(Phase::Delivering, None);
            live.expires_at_ms = deadline;
        }) {
            return;
        }
        self.ack();
        let deliver = json!({
            "op": "deliver",
            "podId": issued.pod_id,
            "refreshToken": issued.refresh_token,
            "scopeId": scope_id,
            "cloudApi": endpoints.api,
            "cloudWs": endpoints.ws,
        });
        match client.call_json(POD_CREDENTIAL_ENDPOINT, &deliver).await {
            Ok(response) if response.get("ok").and_then(Value::as_bool) == Some(true) => {
                if self.enter(Phase::Activating, None) {
                    self.activate(client).await;
                }
            }
            Ok(response) => self.fail(error(
                CommissionErrorCode::DeliveryFailed,
                "The pod did not accept the credential.",
                response
                    .get("reason")
                    .or_else(|| response.get("state"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
            )),
            Err(failure) => {
                warn!(%failure, "credential delivery lost; asking MeowLink");
                if !self.over() {
                    self.settle_with_cloud().await;
                }
            }
        }
    }

    fn deadline(&self) -> i64 {
        self.credential.as_ref().map_or(0, |credential| {
            credential.activate_before + ACTIVATION_GRACE_MS
        })
    }

    async fn activate(&mut self, client: Arc<device::Client>) {
        loop {
            if self.engine.now_ms() >= self.deadline() {
                return self.settle_with_cloud().await;
            }
            if self.sleep(TICK).await.is_none() {
                return;
            }
            let Some(status) = self
                .race(client.call_json(POD_CREDENTIAL_ENDPOINT, &json!({ "op": "status" })))
                .await
            else {
                return;
            };
            let status = match status {
                Ok(status) => status,
                Err(failure) => {
                    warn!(%failure, "pod activation status lost; asking MeowLink");
                    return self.settle_with_cloud().await;
                }
            };
            match status.get("state").and_then(Value::as_str) {
                Some("commissioned") => return self.complete_pod().await,
                Some("failed") => {
                    let code = status
                        .pointer("/error/code")
                        .and_then(Value::as_str)
                        .unwrap_or(NOT_ACTIVATED_DETAIL)
                        .to_string();
                    return self.activation_failed(code);
                }
                _ => {}
            }
        }
    }

    fn activation_failed(&self, detail: String) {
        self.fail(error(
            CommissionErrorCode::ActivationFailed,
            "The pod could not connect to MeowLink.",
            Some(detail),
        ));
    }

    async fn settle_with_cloud(&mut self) {
        let cloud = self.engine.platform.cloud.clone();
        let Some((pod_id, context)) = self
            .credential
            .as_ref()
            .map(|credential| (credential.pod_id.clone(), credential.context.clone()))
        else {
            return;
        };
        loop {
            let Some(status) = self.race(cloud::status(&cloud, &context, &pod_id)).await else {
                return;
            };
            match status.map(|status| status.status) {
                Ok(CredentialStatus::Active) => return self.complete_pod().await,
                Ok(CredentialStatus::Absent) => {
                    if let Some(credential) = self.credential.as_mut() {
                        credential.settled = true;
                    }
                    return self.activation_failed(NOT_ACTIVATED_DETAIL.into());
                }
                Ok(CredentialStatus::Pending) => {}
                Err(failure) => warn!(%failure, "credential status unavailable"),
            }
            if self.engine.now_ms() >= self.deadline() {
                return self.activation_failed(NOT_ACTIVATED_DETAIL.into());
            }
            let poll = Duration::from_millis(CREDENTIAL_STATUS_POLL_MS as u64);
            if self.sleep(poll).await.is_none() {
                return;
            }
        }
    }

    async fn complete_pod(&mut self) {
        let Some(pod_id) = self
            .credential
            .as_ref()
            .map(|credential| credential.pod_id.clone())
        else {
            return;
        };
        let completed = self.update(|live| {
            live.set(Phase::Completed, None);
            live.pod_id = Some(pod_id);
        });
        if !completed {
            return;
        }
        if let Some(credential) = self.credential.as_mut() {
            credential.settled = true;
        }
        if let Some(client) = self.client() {
            self.finish_pod(&client).await;
        }
    }

    async fn cleanup(&mut self) {
        if let Some(device) = self.device.take() {
            device.link.disconnect().await;
        }
        self.engine.release(self.kind, &self.id);
        let Some(credential) = self
            .credential
            .take()
            .filter(|credential| !credential.settled)
        else {
            return;
        };
        let cloud = self.engine.platform.cloud.clone();
        if let Err(failure) = cloud::revoke(&cloud, &credential.context, &credential.pod_id).await {
            warn!(%failure, "revoking an unused pod credential failed");
        }
    }
}
