//! The task that owns one session's link and runs its device and cloud steps
//! in order.
use std::future::Future;
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::time::Duration;

use app_facade_api::host_provision::{is_claim_token, HostFinishResponse, HOST_INFO_ENDPOINT};
use app_facade_api::pod::{
    AuthorizationPrompt, ClientMode, CommissionError, CommissionErrorCode, DeviceKind,
    DiscoveryTransport, PodEnrollmentRequest, PodInfo, SessionMode, SpaceCredentialIssueResponse,
    SpaceCredentialState, ACTIVATION_GRACE_MS, CODE_ATTEMPTS, NOT_ACTIVATED_DETAIL,
    POD_CREDENTIAL_ENDPOINT, POD_INFO_ENDPOINT, SPACE_CREDENTIAL_ISSUE_PATH, SRP_USERNAME,
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
    client_mode: ClientMode,
    confirm_before: i64,
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
    pairing_code: Option<zeroize::Zeroizing<String>>,
    lan: bool,
    lan_addresses: Vec<std::net::SocketAddr>,
    confirmation_retries: u8,
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
            pairing_code: None,
            lan: false,
            lan_addresses: Vec::new(),
            confirmation_retries: 0,
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
        // Keep ownership through connection/version setup, even if the session
        // deadline expires. Cleanup must receive and close a late native link.
        let lan = self
            .engine
            .discovery
            .lock()
            .unwrap()
            .lan
            .get(&self.peripheral_id)
            .cloned();
        let opened = if let Some(lan) = lan {
            self.lan = true;
            self.lan_addresses = lan.addresses.clone();
            self.open_lan_addresses(&lan.addresses).await
        } else if self.peripheral_id.starts_with("lan:") {
            Err(OpenError::Link("Pod is no longer on this LAN".into()))
        } else {
            device::open(
                &self.engine.platform.central,
                &self.peripheral_id,
                self.kind,
            )
            .await
        };
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

    async fn open_lan_addresses(
        &self,
        addresses: &[std::net::SocketAddr],
    ) -> Result<Opened, OpenError> {
        let mut last = OpenError::Link("Pod is no longer on this LAN".into());
        for address in addresses.iter().take(4) {
            if self.over() {
                return Err(OpenError::Link("initialization cancelled".into()));
            }
            let link = match self.engine.platform.lan.connect(*address).await {
                Ok(link) => link,
                Err(e) => {
                    last = OpenError::Link(e.to_string());
                    continue;
                }
            };
            match device::open_link(link, DeviceKind::Pod).await {
                Ok(opened) => return Ok(opened),
                Err(e) => last = e,
            }
        }
        Err(last)
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
                client_mode,
                endpoints,
            } => self.authorize(scope_id, client_mode, endpoints).await,
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
                self.pairing_code = Some(zeroize::Zeroizing::new(code));
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
                if self.lan {
                    let expected = self
                        .engine
                        .mutate(self.kind, &self.id, |live| live.candidate.device_id.clone())
                        .flatten();
                    if expected.as_deref() != Some(&pod_info.device_id)
                        || pod_info.env != core_api::RuntimeEnv::current().as_str()
                    {
                        return self.fail(error(
                            CommissionErrorCode::DeviceMismatch,
                            "This is a different device. Select the pod again.",
                            None,
                        ));
                    }
                }
                let reprovision = pod_info.reprovision();
                let kept_wifi = device.wifi_configured && !reprovision;
                let completed = pod_info.state == "commissioned";
                let activating = pod_info.state == "activating";
                if self.lan && (completed || activating) {
                    let Some(pod_id) = pod_info.pod_id.clone().filter(|id| !id.is_empty()) else {
                        return self.fail(incomplete());
                    };
                    let Some(client_mode) = pod_info.client_mode else {
                        return self.fail(incomplete());
                    };
                    self.credential = Some(Credential {
                        pod_id,
                        client_mode,
                        confirm_before: self.engine.now_ms() + 120_000,
                    });
                }
                self.pod_info = Some(pod_info);
                if !self.update(|live| {
                    live.device = Some(device);
                    live.mode = reprovision.then_some(SessionMode::Reprovision);
                }) {
                    return;
                }
                if self.lan && completed {
                    self.complete_pod().await;
                } else if self.lan && activating {
                    self.enter(Phase::Activating, None);
                    self.activate(client).await;
                } else if self.lan {
                    self.prompt_authorization().await;
                } else if kept_wifi {
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
                if self.kind == DeviceKind::Pod {
                    return self.fail(error(
                        CommissionErrorCode::WifiFailed,
                        "The pod could not join Wi-Fi. Start setup again.",
                        None,
                    ));
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
                    self.enter(
                        if self.kind == DeviceKind::Pod {
                            Phase::Failed
                        } else {
                            Phase::AwaitingWifi
                        },
                        Some(wifi_error(self.kind, failure)),
                    );
                    return;
                }
                Err(failure) => return self.fail_disconnected(failure),
            }
        }
        self.join_failed = true;
        self.enter(
            if self.kind == DeviceKind::Pod {
                Phase::Failed
            } else {
                Phase::AwaitingWifi
            },
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
        if self.kind == DeviceKind::Pod
            && !self.lan
            && (!self.confirm_saved_network().await || !self.handoff_to_lan().await)
        {
            return;
        }
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

    async fn confirm_saved_network(&self) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        for _ in 0..30 {
            let Some(info) = self
                .race(client.call_json(POD_INFO_ENDPOINT, &json!({})))
                .await
            else {
                return false;
            };
            match info {
                Ok(info) if info["state"] == "wifi_connected" => return true,
                Ok(info) if info["state"] == "failed" => break,
                Ok(_) => {}
                Err(error) => {
                    self.fail_disconnected(error);
                    return false;
                }
            }
            if self.sleep(TICK).await.is_none() {
                return false;
            }
        }
        self.fail(error(
            CommissionErrorCode::WifiFailed,
            "The pod did not confirm saving its network. Start setup again.",
            None,
        ));
        false
    }

    /// Network configuration is already committed by the Pod. Dropping BLE now
    /// must not undo it. A failure ends this attempt; the LAN candidate remains.
    async fn handoff_to_lan(&mut self) -> bool {
        let (Some(expected), Some(code)) = (self.pod_info.clone(), self.pairing_code.clone())
        else {
            self.fail_disconnected("pairing context is missing");
            return false;
        };
        if let Some(device) = self.device.take() {
            device.link.disconnect().await;
        }
        if !self.update(|live| {
            live.set(Phase::DiscoveringLan, None);
            live.candidate.wifi_configured = true;
            live.candidate.device_id = Some(expected.device_id.clone());
        }) {
            return false;
        }
        self.engine.reconcile();
        let mut candidates = Vec::new();
        for _ in 0..30 {
            candidates = self
                .engine
                .discovery
                .lock()
                .unwrap()
                .lan
                .values()
                .filter(|a| a.device_id == expected.device_id)
                .cloned()
                .collect();
            if !candidates.is_empty() {
                break;
            }
            if self.sleep(TICK).await.is_none() {
                return false;
            }
        }
        if candidates.is_empty() {
            self.fail(error(
                CommissionErrorCode::LanUnavailable,
                "Wi-Fi is configured. Connect this app to the same local network and try again.",
                None,
            ));
            return false;
        }
        if !self.enter(Phase::ConnectingLan, None) {
            return false;
        }
        for candidate in candidates {
            let Ok(opened) = self.open_lan_addresses(&candidate.addresses).await else {
                continue;
            };
            let client = opened.client.clone();
            self.lan = true;
            self.device = Some(opened);
            // A fresh SRP exchange generates a new key and new record nonces.
            let Some(result) = self
                .race(client.establish_sec2(SRP_USERNAME, &code, 1))
                .await
            else {
                return false;
            };
            if result.is_err() {
                if let Some(device) = self.device.take() {
                    device.link.disconnect().await;
                }
                self.fail(error(
                    CommissionErrorCode::CodeRejected,
                    "The pairing code is no longer valid. Select the pod again.",
                    None,
                ));
                return false;
            }
            let Some(response) = self
                .race(client.call_json(POD_INFO_ENDPOINT, &json!({})))
                .await
            else {
                return false;
            };
            let info = response.ok().and_then(|v| device::pod_info(&v));
            let Some((info, device_info)) = info else {
                if let Some(device) = self.device.take() {
                    device.link.disconnect().await;
                }
                continue;
            };
            if info.device_id != expected.device_id
                || info.identity != expected.identity
                || info.env != core_api::RuntimeEnv::current().as_str()
            {
                if let Some(device) = self.device.take() {
                    device.link.disconnect().await;
                }
                self.fail(error(
                    CommissionErrorCode::DeviceMismatch,
                    "The device identity changed during setup.",
                    None,
                ));
                return false;
            }
            self.lan = true;
            self.lan_addresses = candidate.addresses;
            self.pod_info = Some(info);
            self.update(|live| {
                live.device = Some(device_info);
                live.candidate.transport = DiscoveryTransport::Lan;
                live.candidate.device_id = Some(expected.device_id.clone());
                live.candidate.wifi_configured = true;
            });
            return true;
        }
        self.fail(error(
            CommissionErrorCode::LanUnavailable,
            "Wi-Fi is configured, but the pod could not be reached on this local network.",
            None,
        ));
        false
    }

    async fn authorize(
        &mut self,
        scope_id: String,
        client_mode: ClientMode,
        endpoints: CloudEndpoints,
    ) {
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
        let dependent = info
            .device_binding
            .as_ref()
            .is_some_and(|binding| binding.host.is_some());
        if dependent {
            if client_mode != ClientMode::Space
                || !account
                    .scopes
                    .iter()
                    .any(|s| s.scope_id == scope_id && s.owner)
            {
                return self.fail(issue_failed(Some(
                    "Choose a Space you own for this device".into(),
                )));
            }
            self.ack();
            let prepared = self
                .race(crate::product::prepare(
                    &cloud,
                    &account.context,
                    &scope_id,
                    &info,
                    &client,
                ))
                .await;
            match prepared {
                Some(Ok(())) => {}
                Some(Err(error)) => return self.fail(issue_failed(Some(error))),
                None => return,
            }
        }
        let request = PodEnrollmentRequest {
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
        let (credential, deliver) = match client_mode {
            ClientMode::User => {
                let issued = match cloud::approve_login(&cloud, &account.context, &request).await {
                    Ok(issued) => issued,
                    Err(failure) => return self.fail(issue_failed(Some(failure.to_string()))),
                };
                let deliver = json!({"op":"deliver", "clientMode":"user", "podId":issued.pod_id,
                    "loginGrant":issued.login_grant, "scopeId":scope_id, "cloudApi":endpoints.api, "cloudWs":endpoints.ws});
                (
                    Credential {
                        pod_id: issued.pod_id,
                        client_mode,
                        confirm_before: issued.activate_before + ACTIVATION_GRACE_MS,
                    },
                    deliver,
                )
            }
            ClientMode::Space => {
                if !account.scopes.iter().any(|scope| {
                    scope.scope_id == scope_id && (scope.local || dependent) && scope.owner
                }) {
                    return self.fail(issue_failed(Some("Choose a local Space you own".into())));
                }
                let issued: SpaceCredentialIssueResponse = match cloud::post(
                    &cloud,
                    &account.context,
                    SPACE_CREDENTIAL_ISSUE_PATH,
                    Some(&scope_id),
                    &request,
                )
                .await
                {
                    Ok(issued) => issued,
                    Err(failure) => return self.fail(issue_failed(Some(failure.to_string()))),
                };
                if issued.hub_identity.scope_id != scope_id {
                    return self.fail(issue_failed(Some("Space binding mismatch".into())));
                }
                if !valid_space_credential_window(&issued, self.engine.now_ms()) {
                    return self.fail(issue_failed(Some(
                        "Invalid Space credential activation window".into(),
                    )));
                }
                let deliver = json!({"op":"deliver", "clientMode":"space", "credentialId":issued.credential_id,
                    "appClientId":issued.app_client_id, "jwtToken":issued.jwt_token, "scopeId":scope_id,
                    "tenantId":issued.hub_identity.tenant_id, "hubIdentity":issued.hub_identity});
                (
                    Credential {
                        pod_id: issued.credential_id,
                        client_mode,
                        confirm_before: self.engine.now_ms() + 120_000,
                    },
                    deliver,
                )
            }
        };
        let deadline = credential.confirm_before;
        self.credential = Some(credential);
        if !self.update(|live| {
            live.set(Phase::Delivering, None);
            live.expires_at_ms = deadline;
        }) {
            return;
        }
        self.ack();
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
                // Delivery is not replayed: the Pod may already have committed it.
                if let Some(client) = self.reconnect_for_confirmation().await {
                    if self.enter(Phase::Activating, None) {
                        self.activate(client).await;
                    }
                } else if !self.over() {
                    self.fail(error(
                        CommissionErrorCode::DeliveryFailed,
                        "The pod could not confirm setup. Check its screen before trying again.",
                        Some(failure.to_string()),
                    ));
                }
            }
        }
    }

    /// Init mDNS disappears as soon as activation commits. Reconnect only to
    /// this attempt's endpoint and recheck its authenticated identity; never
    /// issue/deliver another credential while resolving an uncertain result.
    async fn reconnect_for_confirmation(&mut self) -> Option<Arc<device::Client>> {
        if !self.lan {
            return None;
        }
        let expected = self.pod_info.clone()?;
        let code = self.pairing_code.clone()?;
        let pod_id = self.credential.as_ref()?.pod_id.clone();
        let client_mode = self.credential.as_ref()?.client_mode;
        while self.confirmation_retries < 3 && !self.over() {
            self.confirmation_retries += 1;
            if let Some(device) = self.device.take() {
                device.link.disconnect().await;
            }
            self.sleep(TICK).await?;
            let attempt = async {
                let opened = self.open_lan_addresses(&self.lan_addresses).await.ok()?;
                let client = opened.client.clone();
                let patch = opened.patch_version;
                self.device = Some(opened);
                client
                    .establish_sec2(SRP_USERNAME, &code, patch)
                    .await
                    .ok()?;
                let value = client.call_json(POD_INFO_ENDPOINT, &json!({})).await.ok()?;
                Some((client, device::pod_info(&value)?.0))
            };
            let result = tokio::time::timeout(Duration::from_secs(8), attempt).await;
            let Ok(Some((client, info))) = result else {
                continue;
            };
            if info.device_id != expected.device_id
                || info.identity != expected.identity
                || info.env != core_api::RuntimeEnv::current().as_str()
                || info.pod_id.as_deref() != Some(&pod_id)
                || info.client_mode != Some(client_mode)
            {
                self.fail(error(
                    CommissionErrorCode::DeviceMismatch,
                    "The device identity or activation changed during setup.",
                    None,
                ));
                return None;
            }
            if !matches!(
                info.state.as_str(),
                "activating" | "commissioned" | "failed"
            ) {
                return None;
            }
            return Some(client);
        }
        None
    }

    fn deadline(&self) -> i64 {
        self.credential.as_ref().map_or_else(
            || {
                self.engine
                    .mutate(self.kind, &self.id, |live| live.expires_at_ms)
                    .unwrap_or(0)
            },
            |credential| credential.confirm_before,
        )
    }

    async fn activate(&mut self, mut client: Arc<device::Client>) {
        loop {
            if self.engine.now_ms() >= self.deadline() {
                return self.activation_failed(NOT_ACTIVATED_DETAIL.into());
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
                    if let Some(reconnected) = self.reconnect_for_confirmation().await {
                        client = reconnected;
                        continue;
                    }
                    if !self.over() {
                        self.fail_disconnected(failure);
                    }
                    return;
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

    async fn complete_pod(&mut self) {
        let Some(credential) = self.credential.as_ref() else {
            return self.fail_disconnected("missing credential receipt");
        };
        let pod_id = credential.pod_id.clone();
        let client_mode = credential.client_mode;
        let completed = self.update(|live| {
            live.set(Phase::Completed, None);
            live.pod_id = Some(pod_id);
            live.client_mode = Some(client_mode);
        });
        if !completed {
            return;
        }
        if let Some(client) = self.client() {
            self.finish_pod(&client).await;
        }
    }

    async fn cleanup(&mut self) {
        if let Some(device) = self.device.take() {
            if self.lan && device.client.is_secured().await {
                let _ = tokio::time::timeout(
                    Duration::from_secs(2),
                    device
                        .client
                        .call_json(POD_CREDENTIAL_ENDPOINT, &json!({"op":"close"})),
                )
                .await;
            }
            device.link.disconnect().await;
        }
        self.engine.release(self.kind, &self.id);
        self.credential = None;
        self.pairing_code = None;
    }
}

// Credential lifetime and this foreground commissioning attempt are independent.
fn valid_space_credential_window(issued: &SpaceCredentialIssueResponse, now: i64) -> bool {
    match (issued.credential_state, issued.activate_before) {
        (SpaceCredentialState::Pending, Some(deadline)) => deadline > now,
        (SpaceCredentialState::Active, None) => true,
        _ => false,
    }
}
