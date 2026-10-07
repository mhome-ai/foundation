mod support;

use std::future::Future;
use std::sync::Arc;
use std::time::{Duration, Instant};

use app_facade_api::pod::{AdapterStatus, DeviceKind};
use ble_commission::{
    Account, BlockingCentral, BlockingCloud, BlockingLink, BlockingRadio, Central, Cloud,
    CloudEndpoints, CloudError, CloudResponse, Commissioning, Link, LinkError, Platform, Radio,
};
use serde_json::{json, Value};
use support::{FakeCentral, FakeCloud, FakeDevice, FakeRadio, FakeSink, Journal, Script, CODE};

/// Drives the async fakes from the platform's blocking threads.
#[derive(Clone)]
struct Foreign(Arc<tokio::runtime::Runtime>);

impl Foreign {
    fn run<R>(&self, future: impl Future<Output = R>) -> R {
        self.0.block_on(future)
    }
}

struct ForeignRadio(Foreign, Arc<FakeRadio>);
struct ForeignCentral(Foreign, Arc<FakeCentral>);
struct ForeignLink(Foreign, Arc<dyn Link>);
struct ForeignCloud(Foreign, Arc<FakeCloud>);

impl BlockingRadio for ForeignRadio {
    fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus> {
        self.0.run(self.1.set_scanning(scanning))
    }
}

impl BlockingCentral for ForeignCentral {
    fn connect(
        &self,
        peripheral_id: String,
        kind: DeviceKind,
    ) -> Result<Arc<dyn BlockingLink>, LinkError> {
        let link = self.0.run(self.1.connect(&peripheral_id, kind))?;
        Ok(Arc::new(ForeignLink(self.0.clone(), link)))
    }
}

impl BlockingLink for ForeignLink {
    fn exchange(&self, endpoint: String, request: Vec<u8>) -> Result<Vec<u8>, LinkError> {
        self.0.run(self.1.exchange(&endpoint, &request))
    }
    fn is_connected(&self) -> bool {
        self.0.run(self.1.is_connected())
    }
    fn disconnect(&self) {
        self.0.run(self.1.disconnect())
    }
}

impl BlockingCloud for ForeignCloud {
    fn account(&self) -> Option<Account> {
        self.0.run(self.1.account())
    }
    fn post_json(
        &self,
        account_context: String,
        path: String,
        scope_id: Option<String>,
        body: String,
    ) -> Result<CloudResponse, CloudError> {
        self.0.run(
            self.1
                .post_json(&account_context, &path, scope_id.as_deref(), &body),
        )
    }
    fn client_id(&self) -> Option<String> {
        self.0.run(self.1.client_id())
    }
    fn cloud_endpoints(&self) -> CloudEndpoints {
        self.0.run(self.1.cloud_endpoints())
    }
}

fn call(core: &Commissioning, target: &str, body: Value) -> Value {
    let response = core
        .handle_blocking(target, &body.to_string())
        .unwrap_or_else(|error| panic!("{target} failed: {error:?}"));
    serde_json::from_str(&response).unwrap()
}

fn wait(core: &Commissioning, state: &str) -> Value {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let session = call(core, "/local/pod/commission/status", json!({}))["session"].clone();
        if session["state"] == state {
            return session;
        }
        assert!(Instant::now() < deadline, "stuck in {session}");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_blocking_platform_commissions_a_pod_on_its_own_runtime() {
    let foreign = Foreign(Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .enable_time()
            .build()
            .unwrap(),
    ));
    let journal: Journal = Arc::default();
    let radio = Arc::new(FakeRadio::default());
    let central = Arc::new(FakeCentral::default());
    let cloud = Arc::new(FakeCloud::new(journal.clone()));
    let sink = Arc::new(FakeSink::new(journal.clone()));
    let device = FakeDevice::new(Script::pod(), journal.clone());
    device.attach(cloud.clone());
    central
        .devices
        .lock()
        .unwrap()
        .insert("pod-a".into(), device);
    let core = Commissioning::with_own_runtime(Platform::blocking(
        Arc::new(ForeignRadio(foreign.clone(), radio.clone())),
        Arc::new(ForeignCentral(foreign.clone(), central)),
        Arc::new(ForeignCloud(foreign.clone(), cloud.clone())),
        sink.clone(),
    ))
    .unwrap();
    core.on_adapter_state(support::ready());
    core.on_advertisement(support::advertisement("pod-a", DeviceKind::Pod, false));

    let lease = call(&core, "/local/pod/discovery/start", json!({}));
    assert!(lease["leaseId"].is_string());
    let id = call(
        &core,
        "/local/pod/commission/start",
        json!({ "candidateId": "pod-a" }),
    )["sessionId"]
        .as_str()
        .unwrap()
        .to_string();
    wait(&core, "awaiting_code");
    call(
        &core,
        "/local/pod/commission/code",
        json!({ "sessionId": id, "code": CODE }),
    );
    wait(&core, "awaiting_wifi");
    call(
        &core,
        "/local/pod/commission/wifi",
        json!({ "sessionId": id, "ssid": "home", "password": "correct horse" }),
    );
    wait(&core, "awaiting_authorization");
    call(
        &core,
        "/local/pod/commission/authorize",
        json!({ "sessionId": id, "scopeId": "scope-a" }),
    );
    let done = wait(&core, "completed");
    assert_eq!(done["podId"], "pod-1");
    assert!(radio.calls.lock().unwrap().contains(&true));
    assert!(!sink.events.lock().unwrap().is_empty());
    assert_eq!(cloud.paths()[0], "pod/credential/issue");
}
