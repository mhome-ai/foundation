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
// Linux products bound activation (60s), address lookup (20s), Hub proof
// (30s), and failed-trial cleanup/restore (80s). Do not abandon the protocol
// while its backend still owns a potentially disruptive network transaction.
const POD_JOIN_POLLS: u32 = 200;
const HOST_JOIN_POLLS: u32 = 200;
/// How long a pod that kept its network may stay `connecting` before the
/// commissioner offers new Wi-Fi credentials.
const KEPT_WIFI_POLLS: u32 = 30;
const FINISH_ATTEMPTS: u32 = 3;
/// Give the device its 30-second Hub proof budget after IP readiness, with
/// time to receive the durable outcome and finish the BLE exchange.
const REPROVISION_POLLS: u32 = 40;

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
    dependent_host: bool,
    host_needs_claim: bool,
    resume_first_hub: bool,
    first_hub_host: Option<(String, core_api::host::auth::PublicKey)>,
    host_key: Option<core_api::host::auth::PublicKey>,
    target_hub: Option<app_facade_api::pod::HubIdentity>,
    admission: Option<cloud::Admission>,
    destination_admission: Option<cloud::Admission>,
    selected_lan_address: Option<std::net::SocketAddr>,
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
            dependent_host: false,
            host_needs_claim: false,
            resume_first_hub: false,
            first_hub_host: None,
            host_key: None,
            target_hub: None,
            admission: None,
            destination_admission: None,
            selected_lan_address: None,
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
                _ = cancelled(&mut cancel) => break,
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
            .is_none_or(|phase| phase.is_terminal() || phase == Phase::Cancelling)
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

    async fn check_admission(&self) -> bool {
        let cloud = &self.engine.platform.cloud;
        for admission in [&self.admission, &self.destination_admission]
            .into_iter()
            .flatten()
        {
            match self
                .race(admission.check(cloud, self.selected_lan_address))
                .await
            {
                Some(Ok(())) => {}
                Some(Err(failure)) => {
                    self.lan_admission_failed(failure);
                    return false;
                }
                None => return false,
            }
        }
        true
    }

    async fn check_hub_selection(&self) -> bool {
        let cloud = &self.engine.platform.cloud;
        for admission in [&self.admission, &self.destination_admission]
            .into_iter()
            .flatten()
        {
            match self.race(admission.check_hub_selection(cloud)).await {
                Some(Ok(())) => {}
                Some(Err(failure)) => {
                    self.lan_admission_failed(failure);
                    return false;
                }
                None => return false,
            }
        }
        true
    }

    fn lan_admission_failed(&self, failure: crate::platform::CloudError) {
        self.fail(error(
            CommissionErrorCode::LanUnavailable,
            "Connect this Client to the selected Space's local Hub network and start again.",
            Some(failure.to_string()),
        ));
    }

    async fn first_pod_admission(&mut self) -> bool {
        let cloud = self.engine.platform.cloud.clone();
        let expected = self
            .engine
            .mutate(self.kind, &self.id, |live| live.account_context.clone());
        let Some(account) = cloud.account().await.filter(|a| {
            expected
                .as_ref()
                .is_some_and(|context| context.as_ref() == Some(&a.context))
        }) else {
            self.lan_admission_failed(crate::platform::CloudError::AccountChanged);
            return false;
        };
        match self
            .race(cloud::Admission::capture(&cloud, &account, None))
            .await
        {
            Some(Ok(admission)) => self.admission = Some(admission),
            Some(Err(failure)) => {
                self.lan_admission_failed(failure);
                return false;
            }
            None => return false,
        }
        self.check_admission().await
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
        &mut self,
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
                Ok(opened) => {
                    self.selected_lan_address = Some(*address);
                    return Ok(opened);
                }
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
            Op::HostAuthorize => self.authorize_host().await,
            Op::Code(code) => self.code(code).await,
            Op::Wifi {
                ssid,
                password,
                enrollment,
            } => self.wifi(ssid, password, enrollment).await,
            Op::Scan { join: true } => {}
            Op::Scan { join: false } => self.scan().await,
            Op::Authorize {
                space_id,
                client_mode,
                first_hub,
                endpoints,
            } => {
                self.authorize(space_id, client_mode, first_hub, endpoints)
                    .await
            }
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
        let kind = self.kind;
        let incomplete = || {
            error(
                CommissionErrorCode::DeviceBusy,
                &format!("The {} returned incomplete information.", noun(kind)),
                Some(info.to_string()),
            )
        };
        match self.kind {
            DeviceKind::Pod => {
                let parsed = if self.lan {
                    device::pod_info(&info).map(|(info, device)| {
                        let dependent = device.dependent_host;
                        (info, device, dependent)
                    })
                } else {
                    device::pod_network_info(&info)
                };
                let Some((pod_info, device, dependent)) = parsed else {
                    return self.fail(incomplete());
                };
                self.dependent_host = dependent;
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
                if !reprovision && !self.first_pod_admission().await {
                    return;
                }
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
                if host.authorization_mode == core_api::host::access::HostMode::SpaceDep {
                    return self.fail(error(
                        CommissionErrorCode::DeviceMismatch,
                        "Add this device through its Pod setup.",
                        None,
                    ));
                }
                let needs_claim = host.onboarding_required;
                {
                    let identity = match self
                        .race(client.call_json(HOST_INFO_ENDPOINT, &json!({"op":"identity"})))
                        .await
                    {
                        Some(Ok(identity)) => identity,
                        Some(Err(failure)) => return self.fail_disconnected(failure),
                        None => return,
                    };
                    let Some(key) = device::host_identity(&identity, &host.host_id) else {
                        return self.fail(incomplete());
                    };
                    self.host_key = Some(key);
                }
                self.host_needs_claim = needs_claim;
                let recover = self.host_needs_claim
                    && info["networkOnline"] == true
                    && host.authorization_mode == core_api::host::access::HostMode::Admin;
                if self.update(|live| live.host = Some(host)) {
                    if recover {
                        // The secure device response identifies an unfinished first claim.
                        // Keep its saved network; a paired finish obtains a fresh claim token.
                        self.enter(Phase::JoiningWifi, None);
                        self.complete_host(client, String::new()).await;
                    } else {
                        self.offer_wifi(None).await;
                    }
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
        let space_host = self
            .engine
            .mutate(self.kind, &self.id, |live| {
                live.host.as_ref().is_some_and(|host| {
                    host.onboarding_required && host.authorization_mode.is_space()
                })
            })
            .unwrap_or(false);
        let authorization = if (self.kind == DeviceKind::Pod && !self.reprovision()) || space_host {
            let Some(prompt) = self.authorization_prompt().await else {
                return;
            };
            Some(prompt)
        } else {
            None
        };
        if self.update(|live| {
            live.set(Phase::AwaitingWifi, failure);
            live.authorization = authorization;
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

    async fn wifi(
        &mut self,
        ssid: String,
        password: String,
        mut enrollment: Option<app_facade_api::pod::EnrollmentTarget>,
    ) {
        let accepted_phase = match self.kind {
            DeviceKind::Host => Phase::JoiningWifi,
            DeviceKind::Pod => Phase::AwaitingWifi,
        };
        if self.engine.phase(self.kind, &self.id) != Some(accepted_phase) {
            return;
        }
        let Some(client) = self.client() else {
            return self.fail_disconnected("no link");
        };
        let join_failed = self.join_failed;
        if join_failed {
            match self.race(client.wifi_reset()).await {
                Some(Ok(())) => {}
                Some(Err(error)) => return self.fail_disconnected(error),
                None => return,
            }
        }
        let host_mode = self
            .engine
            .mutate(self.kind, &self.id, |live| {
                live.host.as_ref().map(|host| host.authorization_mode)
            })
            .flatten();
        let host_space = host_mode.is_some_and(|mode| mode.is_space());
        let dependent = self.dependent_host || host_space;
        let info_endpoint = if self.kind == DeviceKind::Host {
            HOST_INFO_ENDPOINT
        } else {
            POD_INFO_ENDPOINT
        };
        let mut recovery = None;
        let mut own_hub = false;
        if dependent {
            let target = match self
                .race(client.call_json(info_endpoint, &json!({"op":"networkTarget"})))
                .await
            {
                Some(Ok(target)) => target,
                Some(Err(failure)) => return self.fail_disconnected(failure),
                None => return,
            };
            match target["mode"].as_str() {
                Some("unclaimed")
                    if self.host_needs_claim || (self.dependent_host && !self.reprovision()) => {}
                Some("ownHub") => {
                    own_hub = true;
                    if !matches!(self.race(client.call_json(info_endpoint, &json!({"op":"prepareWifi","mode":"ownHub"}))).await,
                        Some(Ok(reply)) if reply["ok"] == true)
                    {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "The Hub could not prepare its network change.",
                            None,
                        ));
                    }
                }
                Some("space" | "firstHub") => {
                    let Some(space) = target["spaceId"].as_str().filter(|s| !s.is_empty()) else {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "The product omitted its Space network target.",
                            None,
                        ));
                    };
                    let cloud = self.engine.platform.cloud.clone();
                    let Some(account) = self.race(cloud.account()).await.flatten() else {
                        return self.lan_admission_failed(crate::platform::CloudError::NotSignedIn);
                    };
                    if !account
                        .spaces
                        .iter()
                        .any(|s| s.space_id == space && s.owner)
                    {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "Sign in as this Space's owner to change its device network.",
                            None,
                        ));
                    }
                    if enrollment.as_ref().is_some_and(|selected| {
                        selected.space_id != space || selected.client_mode != ClientMode::Space
                    }) {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "Continue setup in the Space already bound to this device.",
                            None,
                        ));
                    }
                    let first_hub = target["mode"] == "firstHub";
                    if first_hub {
                        recovery =
                            match crate::first_hub::Intent::from_target(&target) {
                                Ok(intent) => Some(intent),
                                Err(detail) => return self.fail(error(
                                    CommissionErrorCode::WifiFailed,
                                    "The product returned an invalid first-Hub recovery target.",
                                    Some(detail),
                                )),
                            };
                    }
                    enrollment = Some(app_facade_api::pod::EnrollmentTarget {
                        space_id: space.into(),
                        client_mode: ClientMode::Space,
                        first_hub,
                    });
                }
                _ => {
                    return self.fail(error(
                        CommissionErrorCode::WifiFailed,
                        "The product returned an invalid network target.",
                        None,
                    ))
                }
            }
        }
        if own_hub {
            enrollment = None;
        }
        if let Some(target) = enrollment.as_ref() {
            let Some(context) = self
                .engine
                .mutate(self.kind, &self.id, |live| live.account_context.clone())
                .flatten()
            else {
                return;
            };
            let cloud = self.engine.platform.cloud.clone();
            if self
                .race(cloud.account())
                .await
                .flatten()
                .as_ref()
                .map(|a| &a.context)
                != Some(&context)
            {
                return self.fail(error(
                    if self.kind == DeviceKind::Host {
                        CommissionErrorCode::WifiFailed
                    } else {
                        CommissionErrorCode::IssueFailed
                    },
                    "The MeowLink account changed. Start again.",
                    None,
                ));
            }
            if !self.check_admission().await {
                return;
            }
            let Some(account) = cloud.account().await.filter(|a| a.context == context) else {
                return self.lan_admission_failed(crate::platform::CloudError::AccountChanged);
            };
            if target.first_hub {
                let identity = match self
                    .race(client.call_json(
                        info_endpoint,
                        &json!({"op":
                    if self.kind == DeviceKind::Host { "identity" } else { "hostIdentity" }}),
                    ))
                    .await
                {
                    Some(Ok(identity)) => identity,
                    Some(Err(failure)) => return self.fail_disconnected(failure),
                    None => return,
                };
                let host_id = identity["hostId"].as_str().unwrap_or_default();
                let Some(key) =
                    device::host_identity(&identity, host_id).filter(|_| !host_id.is_empty())
                else {
                    return self.fail(error(
                        CommissionErrorCode::DeviceMismatch,
                        "The product did not provide its Host identity.",
                        None,
                    ));
                };
                if self
                    .host_key
                    .as_ref()
                    .is_some_and(|expected| expected != &key)
                {
                    return self.fail(error(
                        CommissionErrorCode::DeviceMismatch,
                        "The paired Host identity changed.",
                        None,
                    ));
                }
                self.first_hub_host = Some((host_id.to_owned(), key));
            }
            match self
                .race(cloud::Admission::capture_product(
                    &cloud,
                    &account,
                    Some(&target.space_id),
                    self.first_hub_host
                        .as_ref()
                        .filter(|_| recovery.is_some())
                        .map(|(id, _)| id.as_str()),
                ))
                .await
            {
                Some(Ok(admission)) => self.destination_admission = Some(admission),
                Some(Err(failure)) => return self.lan_admission_failed(failure),
                None => return,
            }
            self.target_hub = if recovery.is_some() {
                None // Cloud validates that only this original Host is being resumed.
            } else if target.client_mode == ClientMode::Space {
                match self
                    .race(cloud::resolve_setup_hub(&cloud, &context, &target.space_id))
                    .await
                {
                    Some(Ok(Some(hub))) if !target.first_hub => Some(hub),
                    Some(Ok(None)) if dependent && target.first_hub => None,
                    Some(Ok(Some(_))) => {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "This Space already has a Hub. Start setup again and use that Hub.",
                            None,
                        ))
                    }
                    Some(Ok(None) | Err(_)) => {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "The selected Space has no available Hub to verify.",
                            Some("wifi_hub_unverified".into()),
                        ))
                    }
                    None => return,
                }
            } else {
                None
            };
            if self
                .race(cloud.account())
                .await
                .flatten()
                .as_ref()
                .map(|a| &a.context)
                != Some(&context)
            {
                return self.fail(error(
                    if self.kind == DeviceKind::Host {
                        CommissionErrorCode::WifiFailed
                    } else {
                        CommissionErrorCode::IssueFailed
                    },
                    "The MeowLink account changed. Start again.",
                    None,
                ));
            }
            let mut command = json!({"op":"prepareWifi", "mode":match (target.client_mode, self.target_hub.is_some()) {
                (ClientMode::User, _) => "user",
                (ClientMode::Space, false) => "bootstrap",
                (ClientMode::Space, true) => "space",
            }});
            if target.first_hub {
                let (host_id, key) = self
                    .first_hub_host
                    .as_ref()
                    .expect("paired first-Hub identity");
                command = match self
                    .race(crate::first_hub::prepare(
                        &cloud,
                        &context,
                        &target.space_id,
                        host_id,
                        key,
                        recovery.as_ref(),
                    ))
                    .await
                {
                    Some(Ok(command)) => command,
                    Some(Err(detail)) => {
                        return self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "First-Hub network setup was not authorized.",
                            Some(detail),
                        ))
                    }
                    None => return,
                };
            }
            if let Some(hub) = self.target_hub.as_ref() {
                command["hub"] = json!([
                    hub.hub_id,
                    hub.tenant_id,
                    hub.space_id,
                    hub.public_key_x,
                    hub.public_key_y,
                    hub.host_public_key.x,
                    hub.host_public_key.y
                ]);
            }
            if !matches!(self.race(client.call_json(info_endpoint, &command)).await, Some(Ok(reply)) if reply["ok"] == true)
            {
                return self.fail(error(
                    CommissionErrorCode::WifiFailed,
                    "The pod did not accept the Wi-Fi setup target.",
                    None,
                ));
            }
            self.resume_first_hub = recovery.is_some();
            if !self.update(|live| live.enrollment = enrollment) {
                return;
            }
        }
        if host_mode == Some(core_api::host::access::HostMode::Admin)
            && !matches!(self.race(client.call_json(HOST_INFO_ENDPOINT, &json!({"op":"prepareWifi","mode":"user"}))).await,
                Some(Ok(reply)) if reply["ok"] == true)
        {
            return self.fail(error(
                CommissionErrorCode::WifiFailed,
                "The Host did not accept the Wi-Fi setup target.",
                None,
            ));
        }
        if !self.check_hub_selection().await || !self.check_admission().await {
            return;
        }
        let applied = self
            .race(async {
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
                self.enter(
                    Phase::AwaitingWifi,
                    Some(error(
                        CommissionErrorCode::WifiFailed,
                        &format!("The {} could not use this Wi-Fi network.", noun(self.kind)),
                        Some(failure.to_string()),
                    )),
                );
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
                    if self.kind == DeviceKind::Pod && matches!(failure, WifiFailure::Other) {
                        let Some(status) = self
                            .race(client.call_json(POD_INFO_ENDPOINT, &json!({})))
                            .await
                        else {
                            return;
                        };
                        if status.is_ok_and(|info| {
                            info.pointer("/error/code").and_then(Value::as_str)
                                == Some("wifi_hub_unverified")
                        }) {
                            return self.fail(error(
                                CommissionErrorCode::WifiFailed,
                                "The pod could not verify its Hub through the selected Wi-Fi.",
                                Some("wifi_hub_unverified".into()),
                            ));
                        }
                    }
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
                            if code.as_deref() == Some("wifi_hub_unverified") {
                                "The pod could not verify its Hub through the selected Wi-Fi."
                            } else {
                                "The pod could not save the new Wi-Fi network."
                            },
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
        let mut receipt = None;
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
                    receipt = serde_json::from_value::<HostFinishResponse>(response).ok();
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
        if !self.engine.check_host_account().await {
            return;
        }
        let Some(receipt) = receipt.filter(|receipt| {
            receipt.ok
                && match (self.host_needs_claim, receipt.claim_token.as_deref()) {
                    (true, Some(token)) => is_claim_token(token),
                    (false, None) => true,
                    _ => false,
                }
        }) else {
            return self.fail(error(
                CommissionErrorCode::WifiFailed,
                "The Host did not confirm setup. Reconnect to check the Host before continuing.",
                None,
            ));
        };
        let addresses: Vec<String> = Some(ip4_addr)
            .filter(|address| address.parse::<Ipv4Addr>().is_ok())
            .into_iter()
            .collect();
        if let Some(device) = self.device.take() {
            device.link.disconnect().await;
        }
        self.update(|live| {
            live.set(
                if self.host_needs_claim || self.resume_first_hub {
                    Phase::AwaitingAuthorization
                } else {
                    Phase::Completed
                },
                None,
            );
            live.addresses = addresses;
            live.claim_token = receipt.claim_token;
            if !self.host_needs_claim && !self.resume_first_hub {
                live.host_result = live.host.as_ref().map(|host| {
                    app_facade_api::host_provision::ProvisionResult {
                        host_id: host.host_id.clone(),
                        wifi_configured: true,
                        commissioned: true,
                        hub_id: None,
                    }
                });
            }
        });
    }

    async fn authorize_host(&mut self) {
        let Some((host, enrollment, token, context)) = self
            .engine
            .mutate(self.kind, &self.id, |live| {
                Some((
                    live.host.clone()?,
                    live.enrollment.clone(),
                    live.claim_token.clone(),
                    live.account_context.clone()?,
                ))
            })
            .flatten()
        else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "Host setup context is incomplete. Start again.",
                None,
            ));
        };
        let Some(key) = self.host_key.clone() else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "Host setup context is incomplete. Start again.",
                None,
            ));
        };
        let cloud = self.engine.platform.cloud.clone();
        let Some(account) = self
            .race(cloud.account())
            .await
            .flatten()
            .filter(|account| account.context == context)
        else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "The setup account changed. Start again.",
                None,
            ));
        };
        let space = if host.authorization_mode.is_space() {
            let Some(target) = enrollment.as_ref().filter(|target| {
                target.client_mode == ClientMode::Space
                    && account
                        .spaces
                        .iter()
                        .any(|s| s.space_id == target.space_id && s.owner)
            }) else {
                return self.fail(error(
                    CommissionErrorCode::IssueFailed,
                    "Choose a Space you own for this Host.",
                    None,
                ));
            };
            Some(target.space_id.as_str())
        } else {
            None
        };
        let first_hub = enrollment.as_ref().is_some_and(|target| target.first_hub);
        match self
            .race(cloud::Admission::capture_product(
                &cloud,
                &account,
                space,
                first_hub.then_some(host.host_id.as_str()),
            ))
            .await
        {
            Some(Ok(admission)) => self.destination_admission = Some(admission),
            Some(Err(failure)) => return self.host_authorization_failed(failure.to_string()),
            None => return,
        }
        if !self.check_admission().await || !self.check_hub_selection().await {
            return;
        }
        let encoded_key = serde_json::to_string(&key).expect("Host public key JSON");
        self.ack();
        if self.host_needs_claim {
            let Some(token) = token else {
                return self.host_authorization_failed("Host claim handoff is unavailable".into());
            };
            match self
                .race(cloud.claim_paired_host(&context, &host.host_id, &encoded_key, &token, space))
                .await
            {
                Some(Ok(())) => {}
                Some(Err(failure)) => return self.host_authorization_failed(failure.to_string()),
                None => return,
            }
        }
        let mut hub_id = None;
        if first_hub {
            let space = space.expect("validated Space Host enrollment");
            match self
                .race(cloud.setup_product_hub(&context, space, &host.host_id, &encoded_key))
                .await
            {
                Some(Ok(())) => {}
                Some(Err(failure)) => return self.host_authorization_failed(failure.to_string()),
                None => return,
            }
            match self
                .race(cloud::resolve_setup_hub(&cloud, &context, space))
                .await
            {
                Some(Ok(Some(hub)))
                    if hub.host_id == host.host_id && hub.host_public_key == key =>
                {
                    let mut admission = self
                        .destination_admission
                        .clone()
                        .expect("captured destination");
                    match self
                        .race(admission.accept_created_hub(&cloud, &hub, None))
                        .await
                    {
                        Some(Ok(())) => self.destination_admission = Some(admission),
                        Some(Err(failure)) => {
                            return self.host_authorization_failed(failure.to_string())
                        }
                        None => return,
                    }
                    hub_id = Some(hub.hub_id)
                }
                None => return,
                _ => {
                    return self.host_authorization_failed(
                        "The first Hub has not confirmed this Host's identity.".into(),
                    )
                }
            }
        }
        if !self.engine.check_host_account().await {
            return;
        }
        self.update(|live| {
            live.set(Phase::Completed, None);
            live.claim_token = None;
            live.host_result = Some(app_facade_api::host_provision::ProvisionResult {
                host_id: host.host_id,
                wifi_configured: true,
                commissioned: true,
                hub_id,
            });
        });
    }

    fn host_authorization_failed(&self, detail: String) {
        self.enter(
            Phase::AwaitingAuthorization,
            Some(error(
                CommissionErrorCode::IssueFailed,
                "Host setup did not finish. Check the device and retry.",
                Some(detail),
            )),
        );
    }

    async fn prompt_authorization(&mut self) {
        if self.kind == DeviceKind::Pod
            && !self.lan
            && (!self.confirm_saved_network().await || !self.handoff_to_lan().await)
        {
            return;
        }
        let Some(prompt) = self.authorization_prompt().await else {
            return;
        };
        self.update(|live| {
            live.set(Phase::AwaitingAuthorization, None);
            live.authorization = Some(prompt);
        });
    }

    async fn authorization_prompt(&self) -> Option<AuthorizationPrompt> {
        let account = self.race(self.engine.platform.cloud.account()).await?;
        let context = self
            .engine
            .mutate(self.kind, &self.id, |live| live.account_context.clone())
            .flatten()?;
        let account = account.filter(|account| account.context == context);
        let Some(account) = account else {
            self.fail(error(
                if self.kind == DeviceKind::Host {
                    CommissionErrorCode::WifiFailed
                } else {
                    CommissionErrorCode::IssueFailed
                },
                "Sign in to MeowLink before adding this device.",
                None,
            ));
            return None;
        };
        let Some(first) = account.spaces.first() else {
            self.fail(error(
                if self.kind == DeviceKind::Host {
                    CommissionErrorCode::WifiFailed
                } else {
                    CommissionErrorCode::IssueFailed
                },
                "Join a Space in MeowLink before adding this device.",
                None,
            ));
            return None;
        };
        let default_space_id = account
            .active_space_id
            .clone()
            .filter(|active| account.spaces.iter().any(|scope| &scope.space_id == active))
            .unwrap_or_else(|| first.space_id.clone());
        Some(AuthorizationPrompt {
            user_id: account.user_id,
            user_name: account.user_name,
            spaces: account.spaces,
            default_space_id,
        })
    }

    async fn confirm_saved_network(&self) -> bool {
        let Some(client) = self.client() else {
            return false;
        };
        for _ in 0..REPROVISION_POLLS {
            let Some(info) = self
                .race(client.call_json(POD_INFO_ENDPOINT, &json!({})))
                .await
            else {
                return false;
            };
            match info {
                Ok(info) if info["state"] == "wifi_connected" => return true,
                Ok(info) if matches!(info["state"].as_str(), Some("failed" | "wifi_failed")) => {
                    if info.pointer("/error/code").and_then(Value::as_str)
                        == Some("wifi_hub_unverified")
                    {
                        self.fail(error(
                            CommissionErrorCode::WifiFailed,
                            "The pod could not verify its Hub through the selected Wi-Fi.",
                            Some("wifi_hub_unverified".into()),
                        ));
                        return false;
                    }
                    break;
                }
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
                || info
                    .device_binding
                    .as_ref()
                    .is_some_and(|binding| binding.host.is_some())
                    != self.dependent_host
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
        space_id: String,
        client_mode: ClientMode,
        first_hub: bool,
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
            .mutate(self.kind, &self.id, |live| live.account_context.clone())
            .flatten();
        let Some(account) = account.filter(|account| Some(&account.context) == prompted.as_ref())
        else {
            return self.fail(error(
                CommissionErrorCode::IssueFailed,
                "The MeowLink account changed. Start again.",
                None,
            ));
        };
        if !self.check_admission().await {
            return;
        }
        let product_host = if first_hub {
            self.pod_info
                .as_ref()
                .and_then(|i| i.device_binding.as_ref())
                .and_then(|b| b.host.as_ref())
                .map(|h| h.id.clone())
        } else {
            None
        };
        if self.destination_admission.is_none() || product_host.is_some() {
            match self
                .race(cloud::Admission::capture_product(
                    &cloud,
                    &account,
                    Some(&space_id),
                    product_host.as_deref(),
                ))
                .await
            {
                Some(Ok(admission)) => self.destination_admission = Some(admission),
                Some(Err(failure)) => return self.lan_admission_failed(failure),
                None => return,
            }
            if !self.check_admission().await {
                return;
            }
        }
        if !self.check_hub_selection().await || !self.check_admission().await {
            return;
        }
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
                    .spaces
                    .iter()
                    .any(|s| s.space_id == space_id && s.owner)
            {
                return self.fail(issue_failed(Some(
                    "Choose a Space you own for this device".into(),
                )));
            }
            let selection = async {
                if first_hub {
                    cloud
                        .product_hub_selection(
                            &account.context,
                            &space_id,
                            product_host.as_deref().ok_or(
                                crate::platform::CloudError::Unreachable(
                                    "Product Host identity missing".into(),
                                ),
                            )?,
                        )
                        .await
                } else {
                    cloud.selected_hub(&account.context, &space_id).await
                }
            };
            match self.race(selection).await {
                Some(Ok(hub)) if first_hub || hub.is_some() => {
                    if let Some(expected) = self.target_hub.as_ref() {
                        if hub.as_ref().is_none_or(|(tenant, id)| {
                            tenant != &expected.tenant_id || id != &expected.hub_id
                        }) {
                            return self.fail(issue_failed(Some(
                                "The selected Hub changed during setup.".into(),
                            )));
                        }
                    }
                }
                Some(Ok(_)) => {
                    return self.fail(issue_failed(Some(
                        "This Space has no Hub. Choose the first Hub setup.".into(),
                    )))
                }
                Some(Err(error)) => return self.fail(issue_failed(Some(error.to_string()))),
                None => return,
            }
            self.ack();
            let prepared = self
                .race(crate::product::prepare(
                    &cloud,
                    &account.context,
                    &space_id,
                    &info,
                    &client,
                    first_hub,
                    self.first_hub_host.as_ref(),
                ))
                .await;
            match prepared {
                Some(Ok(hub)) => {
                    if let Some(hub) = hub {
                        let Some(admission) = self.destination_admission.as_mut() else {
                            return;
                        };
                        if let Err(failure) = admission
                            .accept_created_hub(&cloud, &hub, self.selected_lan_address)
                            .await
                        {
                            return self.lan_admission_failed(failure);
                        }
                        self.target_hub = Some(hub);
                    }
                }
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
                    "loginGrant":issued.login_grant, "spaceId":space_id, "cloudApi":endpoints.api, "cloudWs":endpoints.ws});
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
                if !account.spaces.iter().any(|scope| {
                    scope.space_id == space_id && (scope.local || dependent) && scope.owner
                }) {
                    return self.fail(issue_failed(Some("Choose a local Space you own".into())));
                }
                let issued: SpaceCredentialIssueResponse = match cloud::post(
                    &cloud,
                    &account.context,
                    SPACE_CREDENTIAL_ISSUE_PATH,
                    Some(&space_id),
                    &request,
                )
                .await
                {
                    Ok(issued) => issued,
                    Err(failure) => return self.fail(issue_failed(Some(failure.to_string()))),
                };
                if issued.hub_identity.space_id != space_id {
                    return self.fail(issue_failed(Some("Space binding mismatch".into())));
                }
                if !valid_space_credential_window(&issued, self.engine.now_ms()) {
                    return self.fail(issue_failed(Some(
                        "Invalid Space credential activation window".into(),
                    )));
                }
                if self
                    .target_hub
                    .as_ref()
                    .is_some_and(|expected| expected != &issued.hub_identity)
                {
                    return self.fail(issue_failed(Some(
                        "The selected Space's Hub changed during setup".into(),
                    )));
                }
                let deliver = json!({"op":"deliver", "clientMode":"space", "credentialId":issued.credential_id,
                    "appClientId":issued.app_client_id, "jwtToken":issued.jwt_token, "spaceId":space_id,
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
        if !self.check_admission().await {
            return;
        }
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
                let addresses = self.lan_addresses.clone();
                let opened = self.open_lan_addresses(&addresses).await.ok()?;
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
                || info
                    .device_binding
                    .as_ref()
                    .is_some_and(|binding| binding.host.is_some())
                    != self.dependent_host
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
