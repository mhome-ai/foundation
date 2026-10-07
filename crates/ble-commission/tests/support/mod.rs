//! A fake platform and a fake pod/Host built on the protocomm device side.
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use app_facade_api::pod::{
    AdapterState, AdapterStatus, AuthorizationScope, BleAdvertisement, DeviceKind,
    BLE_FLAG_COMMISSIONABLE, BLE_FLAG_WIFI_CONFIGURED, BLE_KIND_HOST, BLE_KIND_POD,
};
use async_trait::async_trait;
use ble_commission::{
    Account, Advertisement, Central, Cloud, CloudEndpoints, CloudError, CloudResponse,
    Commissioning, EventSink, Link, LinkError, Platform, Radio,
};
use core_api::ErrorResponse;
use prost::Message;
use protocomm::proto::{self, Status};
use protocomm::{SessionError, SessionResponder};
use serde_json::{json, Value};

pub const CODE: &str = "042137";
pub const CLAIM_TOKEN: &str = "q3hX0v2c1mJb9yQe7tL4nK8sP5wR6uA0zD1fG2hI3jM";

pub type Journal = Arc<Mutex<Vec<String>>>;

fn note(journal: &Journal, entry: impl Into<String>) {
    journal.lock().unwrap().push(entry.into());
}

/// Same arithmetic as the core's clock, so both agree under paused time.
pub struct Clock {
    base_ms: i64,
    start: tokio::time::Instant,
}

impl Clock {
    pub fn new() -> Self {
        Self {
            base_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            start: tokio::time::Instant::now(),
        }
    }

    pub fn now_ms(&self) -> i64 {
        self.base_ms + self.start.elapsed().as_millis() as i64
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Join {
    Connected(&'static str),
    Failed(proto::WifiConnectFailedReason),
    Never,
}

#[derive(Clone, Debug)]
pub enum Activation {
    Commissioned { after_polls: u32 },
    Failed(&'static str),
    Never,
}

pub struct Script {
    pub kind: DeviceKind,
    pub code: &'static str,
    pub wifi_configured: bool,
    pub kept_wifi: Join,
    pub kept_wifi_connecting_polls: u32,
    pub join: Join,
    pub join_connecting_polls: u32,
    pub activation: Activation,
    pub deliver_ok: bool,
    /// Drops the link right after handling this op (`deliver`, `status`...).
    pub drop_after_op: Option<&'static str>,
    pub lock_after_rejections: Option<u32>,
    pub claim_token: Option<&'static str>,
}

impl Script {
    pub fn pod() -> Self {
        Self {
            kind: DeviceKind::Pod,
            code: CODE,
            wifi_configured: false,
            kept_wifi: Join::Connected("192.168.0.50"),
            kept_wifi_connecting_polls: 0,
            join: Join::Connected("192.168.0.50"),
            join_connecting_polls: 2,
            activation: Activation::Commissioned { after_polls: 2 },
            deliver_ok: true,
            drop_after_op: None,
            lock_after_rejections: None,
            claim_token: None,
        }
    }

    pub fn host() -> Self {
        Self {
            kind: DeviceKind::Host,
            claim_token: Some(CLAIM_TOKEN),
            lock_after_rejections: Some(5),
            ..Self::pod()
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Sta {
    Idle,
    Kept { polls: u32 },
    Joining { polls: u32 },
    Done(Join),
}

struct DeviceState {
    responder: SessionResponder,
    sta: Sta,
    configured: Option<(String, String)>,
    pod_state: &'static str,
    activation_polls: u32,
    rejections: u32,
}

pub struct FakeDevice {
    pub script: Mutex<Script>,
    state: Mutex<DeviceState>,
    pub connected: AtomicBool,
    pub locked: AtomicBool,
    pub connects: AtomicU32,
    pub scans: AtomicU32,
    journal: Journal,
    cloud: Mutex<Option<Arc<FakeCloud>>>,
}

impl FakeDevice {
    pub fn new(script: Script, journal: Journal) -> Arc<Self> {
        let (salt, verifier) = protocomm::srp::generate_salt_verifier("meow", script.code);
        let sta = if script.wifi_configured {
            Sta::Kept { polls: 0 }
        } else {
            Sta::Idle
        };
        Arc::new(Self {
            state: Mutex::new(DeviceState {
                responder: SessionResponder::new("meow", salt, verifier),
                sta,
                configured: None,
                pod_state: "advertising",
                activation_polls: 0,
                rejections: 0,
            }),
            script: Mutex::new(script),
            connected: AtomicBool::new(false),
            locked: AtomicBool::new(false),
            connects: AtomicU32::new(0),
            scans: AtomicU32::new(0),
            journal,
            cloud: Mutex::new(None),
        })
    }

    pub fn attach(&self, cloud: Arc<FakeCloud>) {
        *self.cloud.lock().unwrap() = Some(cloud);
    }

    pub fn drop_link(&self) {
        self.connected.store(false, Ordering::SeqCst);
    }

    fn version(&self) -> Value {
        let script = self.script.lock().unwrap();
        let cap = match script.kind {
            DeviceKind::Pod => "pod",
            DeviceKind::Host => "host",
        };
        json!({
            "prov": { "ver": "v1.1", "sec_ver": 2, "sec_patch_ver": 1, "cap": ["wifi_scan"] },
            "meow": { "ver": "1", "cap": [cap], "locked": self.locked.load(Ordering::SeqCst) },
        })
    }

    fn session(&self, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        if self.locked.load(Ordering::SeqCst) {
            return Err(LinkError::Rejected("locked".into()));
        }
        let mut state = self.state.lock().unwrap();
        match state.responder.handle(request) {
            Ok(response) => Ok(response),
            Err(SessionError::ProofRejected) => {
                state.rejections += 1;
                note(&self.journal, "device:rejected");
                let limit = self.script.lock().unwrap().lock_after_rejections;
                if limit.is_some_and(|limit| state.rejections >= limit) {
                    self.locked.store(true, Ordering::SeqCst);
                }
                Err(LinkError::Rejected("proof".into()))
            }
            Err(error) => Err(LinkError::Rejected(error.to_string())),
        }
    }

    fn scan(&self, request: &[u8]) -> Vec<u8> {
        use proto::wifi_scan_payload::Payload;
        let request = proto::WiFiScanPayload::decode(request).unwrap();
        let entry = |ssid: &str, rssi: i32, auth: proto::WifiAuthMode| proto::WiFiScanResult {
            ssid: ssid.as_bytes().to_vec(),
            channel: 6,
            rssi,
            bssid: vec![0; 6],
            auth: auth as i32,
        };
        let payload = match request.payload {
            Some(Payload::CmdScanStart(_)) => {
                self.scans.fetch_add(1, Ordering::SeqCst);
                note(&self.journal, "device:scan");
                Payload::RespScanStart(proto::RespScanStart {})
            }
            Some(Payload::CmdScanStatus(_)) => Payload::RespScanStatus(proto::RespScanStatus {
                scan_finished: true,
                result_count: 2,
            }),
            _ => Payload::RespScanResult(proto::RespScanResult {
                entries: vec![
                    entry("home", -50, proto::WifiAuthMode::Wpa2Psk),
                    entry("cafe", -70, proto::WifiAuthMode::Open),
                ],
            }),
        };
        proto::WiFiScanPayload {
            msg: 0,
            status: Status::Success as i32,
            payload: Some(payload),
        }
        .encode_to_vec()
    }

    fn sta_status(state: &mut DeviceState, script: &Script) -> proto::RespGetStatus {
        use proto::resp_get_status::State;
        let outcome = match &mut state.sta {
            Sta::Idle => None,
            Sta::Kept { polls } => {
                *polls += 1;
                (*polls > script.kept_wifi_connecting_polls).then(|| script.kept_wifi.clone())
            }
            Sta::Joining { polls } => {
                *polls += 1;
                (*polls > script.join_connecting_polls).then(|| script.join.clone())
            }
            Sta::Done(join) => Some(join.clone()),
        };
        if let Some(join) = outcome.clone() {
            state.sta = Sta::Done(join);
        }
        let (sta_state, detail) = match outcome {
            None | Some(Join::Never) => (proto::WifiStationState::Connecting, None),
            Some(Join::Connected(ip)) => (
                proto::WifiStationState::Connected,
                Some(State::Connected(proto::WifiConnectedState {
                    ip4_addr: ip.into(),
                    ..Default::default()
                })),
            ),
            Some(Join::Failed(reason)) => (
                proto::WifiStationState::ConnectionFailed,
                Some(State::FailReason(reason as i32)),
            ),
        };
        if sta_state == proto::WifiStationState::Connected {
            state.pod_state = "wifi_connected";
        }
        proto::RespGetStatus {
            status: Status::Success as i32,
            sta_state: sta_state as i32,
            state: detail,
        }
    }

    fn config(&self, state: &mut DeviceState, request: &[u8]) -> Vec<u8> {
        use proto::wifi_config_payload::Payload;
        let request = proto::WiFiConfigPayload::decode(request).unwrap();
        let script = self.script.lock().unwrap();
        let payload = match request.payload {
            Some(Payload::CmdGetStatus(_)) => {
                Payload::RespGetStatus(Self::sta_status(state, &script))
            }
            Some(Payload::CmdSetConfig(config)) => {
                let busy = matches!(state.sta, Sta::Done(Join::Failed(_)));
                note(&self.journal, "device:set_config");
                state.configured = Some((
                    String::from_utf8(config.ssid).unwrap(),
                    String::from_utf8(config.passphrase).unwrap(),
                ));
                Payload::RespSetConfig(proto::RespSetConfig {
                    status: if busy {
                        Status::InternalError as i32
                    } else {
                        Status::Success as i32
                    },
                })
            }
            _ => {
                note(&self.journal, "device:apply");
                state.sta = Sta::Joining { polls: 0 };
                Payload::RespApplyConfig(proto::RespApplyConfig {
                    status: Status::Success as i32,
                })
            }
        };
        proto::WiFiConfigPayload {
            msg: 0,
            payload: Some(payload),
        }
        .encode_to_vec()
    }

    fn json_endpoint(
        &self,
        state: &mut DeviceState,
        endpoint: &str,
        request: &[u8],
    ) -> (Vec<u8>, String) {
        let request: Value = serde_json::from_slice(request).unwrap_or(Value::Null);
        let op = request
            .get("op")
            .and_then(Value::as_str)
            .unwrap_or("info")
            .to_string();
        let script = self.script.lock().unwrap();
        note(&self.journal, format!("device:{endpoint}:{op}"));
        let response = match (endpoint, op.as_str()) {
            ("pod-info", _) => json!({
                "protocol": 1,
                "deviceId": "a0b1c2d3e4f5",
                "model": "S1",
                "platform": "esp32c5",
                "firmwareVersion": "0.3.0",
                "identity": { "keyId": "p256:abc", "alg": "ES256", "x": "x", "y": "y", "fingerprint": "f" },
                "wifiConfigured": script.wifi_configured,
                "state": "secured",
            }),
            ("pod-credential", "deliver") => {
                if script.deliver_ok {
                    state.pod_state = "activating";
                    json!({ "ok": true, "state": "activating" })
                } else {
                    json!({ "ok": false, "state": state.pod_state })
                }
            }
            ("pod-credential", "status") => {
                if state.pod_state == "activating" {
                    state.activation_polls += 1;
                    match script.activation {
                        Activation::Commissioned { after_polls }
                            if state.activation_polls > after_polls =>
                        {
                            state.pod_state = "commissioned";
                            if let Some(cloud) = self.cloud.lock().unwrap().as_ref() {
                                cloud.activate();
                            }
                        }
                        Activation::Failed(code) => {
                            return (
                                serde_json::to_vec(&json!({
                                    "state": "failed",
                                    "error": { "code": code, "message": "x" },
                                }))
                                .unwrap(),
                                op,
                            )
                        }
                        _ => {}
                    }
                }
                json!({ "state": state.pod_state })
            }
            ("host-info", "info") => json!({
                "protocol": 1,
                "hostId": "host-1",
                "name": "Kitchen Host",
                "firmwareVersion": "1.0.7",
                "fingerprint": "ab:cd",
                "wifiConfigured": false,
            }),
            ("host-info", "finish") => match script.claim_token {
                Some(token) => json!({ "ok": true, "claimToken": token }),
                None => json!({ "ok": true }),
            },
            _ => json!({ "ok": true }),
        };
        (serde_json::to_vec(&response).unwrap(), op)
    }

    fn encrypted(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        let mut guard = self.state.lock().unwrap();
        let state = &mut *guard;
        let plain = state
            .responder
            .decrypt(request)
            .map_err(|error| LinkError::Rejected(error.to_string()))?;
        let mut op = String::new();
        let response = match endpoint {
            "prov-scan" => self.scan(&plain),
            "prov-config" => self.config(state, &plain),
            "prov-ctrl" => {
                note(&self.journal, "device:reset");
                state.sta = Sta::Idle;
                proto::WiFiCtrlPayload {
                    msg: proto::WiFiCtrlMsgType::TypeRespCtrlReset as i32,
                    status: Status::Success as i32,
                    payload: Some(proto::wifi_ctrl_payload::Payload::RespCtrlReset(
                        proto::RespCtrlReset {},
                    )),
                }
                .encode_to_vec()
            }
            _ => {
                let (response, handled) = self.json_endpoint(state, endpoint, &plain);
                op = handled;
                response
            }
        };
        let record = state
            .responder
            .encrypt(&response)
            .map_err(|error| LinkError::Rejected(error.to_string()))?;
        drop(guard);
        if self.script.lock().unwrap().drop_after_op == Some(op.as_str()) && !op.is_empty() {
            self.drop_link();
            return Err(LinkError::Disconnected("response lost".into()));
        }
        Ok(record)
    }
}

#[async_trait]
impl Link for FakeDevice {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        tokio::time::sleep(Duration::from_millis(20)).await;
        if !self.connected.load(Ordering::SeqCst) {
            return Err(LinkError::Disconnected("not connected".into()));
        }
        match endpoint {
            "proto-ver" => Ok(self.version().to_string().into_bytes()),
            "prov-session" => self.session(request),
            _ => self.encrypted(endpoint, request),
        }
    }

    async fn is_connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    async fn disconnect(&self) {
        if self.connected.swap(false, Ordering::SeqCst) {
            note(&self.journal, "device:disconnected");
        }
    }
}

struct DeviceLink(Arc<FakeDevice>);

#[async_trait]
impl Link for DeviceLink {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        self.0.exchange(endpoint, request).await
    }
    async fn is_connected(&self) -> bool {
        self.0.is_connected().await
    }
    async fn disconnect(&self) {
        self.0.disconnect().await
    }
}

#[derive(Default)]
pub struct FakeCentral {
    pub devices: Mutex<HashMap<String, Arc<FakeDevice>>>,
}

#[async_trait]
impl Central for FakeCentral {
    async fn connect(
        &self,
        peripheral_id: &str,
        _kind: DeviceKind,
    ) -> Result<Arc<dyn Link>, LinkError> {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let device = self
            .devices
            .lock()
            .unwrap()
            .get(peripheral_id)
            .cloned()
            .ok_or_else(|| LinkError::NotFound(peripheral_id.into()))?;
        device.state.lock().unwrap().responder.reset();
        device.connected.store(true, Ordering::SeqCst);
        device.connects.fetch_add(1, Ordering::SeqCst);
        Ok(Arc::new(DeviceLink(device)))
    }
}

#[derive(Default)]
pub struct FakeRadio {
    pub calls: Mutex<Vec<bool>>,
    pub refuse: AtomicBool,
}

#[async_trait]
impl Radio for FakeRadio {
    async fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus> {
        self.calls.lock().unwrap().push(scanning);
        if scanning && self.refuse.load(Ordering::SeqCst) {
            return Err(AdapterStatus {
                state: AdapterState::Denied,
                message: Some("denied".into()),
            });
        }
        Ok(())
    }

    async fn open_bluetooth_settings(&self) -> bool {
        true
    }
}

#[derive(Debug, Clone)]
pub struct CloudCall {
    pub context: String,
    pub path: String,
    pub scope_id: Option<String>,
    pub body: Value,
}

pub struct FakeCloud {
    pub account: Mutex<Option<Account>>,
    pub endpoints: Mutex<CloudEndpoints>,
    pub calls: Mutex<Vec<CloudCall>>,
    /// Lion's view of the issued credential.
    pub status: Mutex<&'static str>,
    pub statuses: Mutex<VecDeque<&'static str>>,
    pub issue_delay: Mutex<Duration>,
    pub fail_issue: AtomicBool,
    clock: Clock,
    journal: Journal,
}

impl FakeCloud {
    pub fn new(journal: Journal) -> Self {
        Self {
            account: Mutex::new(Some(account("ctx-1"))),
            endpoints: Mutex::new(CloudEndpoints {
                api: "https://cloud.example/api/v1".into(),
                ws: "wss://cloud.example/ws".into(),
            }),
            calls: Mutex::new(Vec::new()),
            status: Mutex::new("pending"),
            statuses: Mutex::new(VecDeque::new()),
            issue_delay: Mutex::new(Duration::from_millis(100)),
            fail_issue: AtomicBool::new(false),
            clock: Clock::new(),
            journal,
        }
    }

    pub fn activate(&self) {
        *self.status.lock().unwrap() = "active";
    }

    pub fn paths(&self) -> Vec<String> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|call| call.path.clone())
            .collect()
    }

    pub fn revokes(&self) -> Vec<CloudCall> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| call.path == "pod/credential/revoke")
            .cloned()
            .collect()
    }
}

pub fn account(context: &str) -> Account {
    Account {
        user_id: "user-1".into(),
        user_name: "Ada".into(),
        scopes: vec![
            AuthorizationScope {
                scope_id: "scope-a".into(),
                name: "Home".into(),
            },
            AuthorizationScope {
                scope_id: "scope-b".into(),
                name: "Cabin".into(),
            },
        ],
        active_scope_id: Some("scope-b".into()),
        context: context.into(),
    }
}

#[async_trait]
impl Cloud for FakeCloud {
    async fn account(&self) -> Option<Account> {
        self.account.lock().unwrap().clone()
    }

    async fn post_json(
        &self,
        account_context: &str,
        path: &str,
        scope_id: Option<&str>,
        body: &str,
    ) -> Result<CloudResponse, CloudError> {
        self.calls.lock().unwrap().push(CloudCall {
            context: account_context.into(),
            path: path.into(),
            scope_id: scope_id.map(str::to_string),
            body: serde_json::from_str(body).unwrap(),
        });
        note(&self.journal, format!("cloud:{path}"));
        let body = match path {
            "pod/credential/issue" => {
                let delay = *self.issue_delay.lock().unwrap();
                tokio::time::sleep(delay).await;
                if self.fail_issue.load(Ordering::SeqCst) {
                    json!({ "error": "NOT_FOUND", "message": "no such Space" })
                } else {
                    json!({
                        "podId": "pod-1",
                        "refreshToken": "refresh-1",
                        "activateBefore": self.clock.now_ms() + 600_000,
                    })
                }
            }
            "pod/credential/status" => {
                let scripted = self.statuses.lock().unwrap().pop_front();
                let status = scripted.unwrap_or(*self.status.lock().unwrap());
                if status == "unreachable" {
                    return Err(CloudError::Unreachable("offline".into()));
                }
                json!({ "podId": "pod-1", "status": status })
            }
            _ => json!({}),
        };
        Ok(CloudResponse {
            status: 200,
            body: body.to_string(),
        })
    }

    async fn client_id(&self) -> Option<String> {
        Some("client-1".into())
    }

    async fn cloud_endpoints(&self) -> CloudEndpoints {
        self.endpoints.lock().unwrap().clone()
    }
}

pub struct FakeSink {
    pub events: Mutex<Vec<(String, Value)>>,
    journal: Journal,
}

impl FakeSink {
    pub fn new(journal: Journal) -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            journal,
        }
    }
}

impl EventSink for FakeSink {
    fn publish(&self, target: &str, payload: &str) {
        let value: Value = serde_json::from_str(payload).unwrap();
        if let Some(state) = value.get("state").and_then(Value::as_str) {
            note(&self.journal, format!("event:{state}"));
        }
        self.events
            .lock()
            .unwrap()
            .push((target.to_string(), value));
    }
}

pub struct Harness {
    pub core: Commissioning,
    pub radio: Arc<FakeRadio>,
    pub central: Arc<FakeCentral>,
    pub cloud: Arc<FakeCloud>,
    pub sink: Arc<FakeSink>,
    pub journal: Journal,
}

pub fn ready() -> AdapterStatus {
    AdapterStatus {
        state: AdapterState::Ready,
        message: None,
    }
}

impl Harness {
    pub fn new() -> Self {
        let journal: Journal = Arc::default();
        let radio = Arc::new(FakeRadio::default());
        let central = Arc::new(FakeCentral::default());
        let cloud = Arc::new(FakeCloud::new(journal.clone()));
        let sink = Arc::new(FakeSink::new(journal.clone()));
        let platform = Platform {
            radio: radio.clone(),
            central: central.clone(),
            cloud: cloud.clone(),
            events: sink.clone(),
        };
        let core = Commissioning::new(platform, tokio::runtime::Handle::current());
        core.on_adapter_state(ready());
        Self {
            core,
            radio,
            central,
            cloud,
            sink,
            journal,
        }
    }

    pub fn add_device(&self, id: &str, script: Script) -> Arc<FakeDevice> {
        let kind = script.kind;
        let wifi_configured = script.wifi_configured;
        let device = FakeDevice::new(script, self.journal.clone());
        device.attach(self.cloud.clone());
        self.central
            .devices
            .lock()
            .unwrap()
            .insert(id.to_string(), device.clone());
        self.core
            .on_advertisement(advertisement(id, kind, wifi_configured));
        device
    }

    pub fn advertise(&self, id: &str, kind: DeviceKind, flags: u8, rssi: i16) {
        self.core
            .on_advertisement(advertisement_with(id, kind, flags, rssi));
    }

    pub async fn call(&self, target: &str, body: Value) -> Result<Value, ErrorResponse> {
        self.core
            .handle(target, &body.to_string())
            .await
            .map(|response| serde_json::from_str(&response).unwrap())
    }

    pub async fn ok(&self, target: &str, body: Value) -> Value {
        match self.call(target, body).await {
            Ok(value) => value,
            Err(error) => panic!("{target} failed: {error:?}"),
        }
    }

    pub async fn reason(&self, target: &str, body: Value) -> String {
        match self.call(target, body).await {
            Ok(value) => panic!("{target} unexpectedly succeeded: {value}"),
            Err(error) => error
                .details
                .and_then(|details| details["reason"].as_str().map(str::to_string))
                .unwrap_or(error.error),
        }
    }

    /// Polls `status` (renewing the owner lease when `renew`) until `done`.
    pub async fn wait(
        &self,
        kind: DeviceKind,
        renew: bool,
        done: impl Fn(&Value) -> bool,
    ) -> Value {
        let (status, renew_target) = match kind {
            DeviceKind::Pod => (
                "/local/pod/commission/status",
                "/local/pod/commission/renew",
            ),
            DeviceKind::Host => (
                "/local/host/provision/status",
                "/local/host/provision/renew",
            ),
        };
        for _ in 0..20_000 {
            let session = self.ok(status, json!({})).await["session"].clone();
            if done(&session) {
                return session;
            }
            if renew {
                if let Some(id) = session["sessionId"].as_str() {
                    let _ = self.call(renew_target, json!({ "sessionId": id })).await;
                }
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        panic!("session never reached the expected state");
    }

    pub async fn wait_state(&self, kind: DeviceKind, state: &str) -> Value {
        self.wait(kind, true, |session| session["state"] == state)
            .await
    }

    pub fn journal(&self) -> Vec<String> {
        self.journal.lock().unwrap().clone()
    }

    pub fn position(&self, entry: &str) -> usize {
        self.journal()
            .iter()
            .position(|item| item == entry)
            .unwrap_or_else(|| panic!("{entry} not in {:?}", self.journal()))
    }

    pub fn events(&self, target: &str) -> Vec<Value> {
        self.sink
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|(event, _)| event == target)
            .map(|(_, value)| value.clone())
            .collect()
    }
}

pub fn advertisement(id: &str, kind: DeviceKind, wifi_configured: bool) -> Advertisement {
    let mut flags = BLE_FLAG_COMMISSIONABLE;
    if wifi_configured {
        flags |= BLE_FLAG_WIFI_CONFIGURED;
    }
    advertisement_with(id, kind, flags, -55)
}

pub fn advertisement_with(id: &str, kind: DeviceKind, flags: u8, rssi: i16) -> Advertisement {
    let (kind, name) = match kind {
        DeviceKind::Pod => (BLE_KIND_POD, "MeowPod S1"),
        DeviceKind::Host => (BLE_KIND_HOST, "Kitchen Host"),
    };
    Advertisement {
        peripheral_id: id.into(),
        local_name: Some(name.into()),
        manufacturer_data: Some(
            BleAdvertisement {
                kind,
                flags,
                short_id: [0xA1, 0xB2, 0xC3, 0xD4],
            }
            .encode()
            .to_vec(),
        ),
        service_uuids: vec![app_facade_api::pod::BLE_SERVICE_UUID.into()],
        rssi: Some(rssi),
    }
}

pub fn session_id(session: &Value) -> String {
    session["sessionId"].as_str().unwrap().to_string()
}
