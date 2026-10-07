mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use app_facade_api::pod::{
    CommissionSession, DeviceKind, BLE_FLAG_COMMISSIONABLE, COMMISSION_CHANGED_EVENT,
};
use serde_json::{json, Value};
use support::{session_id, Activation, Harness, Script, CODE};

const POD: DeviceKind = DeviceKind::Pod;

async fn start(harness: &Harness) -> String {
    let session = harness
        .ok(
            "/local/pod/commission/start",
            json!({ "candidateId": "pod-a" }),
        )
        .await;
    assert_eq!(session["state"], "connecting");
    session_id(&session)
}

async fn secure(harness: &Harness, id: &str) {
    harness.wait_state(POD, "awaiting_code").await;
    harness
        .ok(
            "/local/pod/commission/code",
            json!({ "sessionId": id, "code": CODE }),
        )
        .await;
}

async fn to_authorization(harness: &Harness) -> String {
    let id = start(harness).await;
    secure(harness, &id).await;
    let session = harness
        .wait(POD, true, |session| {
            session["state"] == "awaiting_wifi" && session["networks"].is_array()
        })
        .await;
    assert_eq!(session["networks"][0]["ssid"], "home");
    harness
        .ok(
            "/local/pod/commission/wifi",
            json!({ "sessionId": id, "ssid": "home", "password": "correct horse" }),
        )
        .await;
    harness.wait_state(POD, "awaiting_authorization").await;
    id
}

async fn authorize(harness: &Harness, id: &str) -> Value {
    harness
        .ok(
            "/local/pod/commission/authorize",
            json!({ "sessionId": id, "scopeId": "scope-a" }),
        )
        .await
}

fn valid(session: &Value) {
    let parsed: CommissionSession = serde_json::from_value(session.clone()).unwrap();
    parsed.validate().unwrap();
}

#[tokio::test(start_paused = true)]
async fn commissions_a_pod_end_to_end() {
    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    let id = to_authorization(&harness).await;
    let prompt = harness.wait_state(POD, "awaiting_authorization").await;
    assert_eq!(prompt["authorization"]["defaultScopeId"], "scope-b");
    assert_eq!(prompt["authorization"]["userName"], "Ada");
    let issued = authorize(&harness, &id).await;
    assert_eq!(issued["state"], "delivering");
    let done = harness.wait_state(POD, "completed").await;
    assert_eq!(done["podId"], "pod-1");
    valid(&done);

    assert_eq!(device.scans.load(Ordering::SeqCst), 1);
    assert_eq!(device.connects.load(Ordering::SeqCst), 1);
    let calls = harness.cloud.calls.lock().unwrap().clone();
    assert_eq!(calls[0].path, "pod/credential/issue");
    assert_eq!(calls[0].scope_id.as_deref(), Some("scope-a"));
    assert_eq!(calls[0].context, "ctx-1");
    assert_eq!(calls[0].body["clientId"], "client-1");
    assert_eq!(calls[0].body["identity"]["keyId"], "p256:abc");
    assert!(harness.cloud.revokes().is_empty());
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(harness.position("event:completed") < harness.position("device:pod-credential:finish"));
    assert!(harness
        .journal()
        .contains(&"device:disconnected".to_string()));

    let events = harness.events(COMMISSION_CHANGED_EVENT);
    let revisions: Vec<u64> = events
        .iter()
        .map(|event| event["revision"].as_u64().unwrap())
        .collect();
    assert!(revisions.windows(2).all(|pair| pair[0] < pair[1]));
    events.iter().for_each(valid);
}

#[tokio::test(start_paused = true)]
async fn a_wrong_code_can_be_retried_on_the_same_link() {
    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    let id = start(&harness).await;
    harness.wait_state(POD, "awaiting_code").await;
    let rejected = harness
        .ok(
            "/local/pod/commission/code",
            json!({ "sessionId": id, "code": "000000" }),
        )
        .await;
    assert_eq!(rejected["state"], "awaiting_code");
    assert_eq!(rejected["error"]["code"], "code_rejected");
    harness
        .ok(
            "/local/pod/commission/code",
            json!({ "sessionId": id, "code": CODE }),
        )
        .await;
    harness.wait_state(POD, "awaiting_wifi").await;
    assert_eq!(device.connects.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn five_wrong_codes_lock_the_session() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    let id = start(&harness).await;
    harness.wait_state(POD, "awaiting_code").await;
    let mut last = Value::Null;
    for _ in 0..5 {
        last = harness
            .ok(
                "/local/pod/commission/code",
                json!({ "sessionId": id, "code": "111111" }),
            )
            .await;
    }
    assert_eq!(last["state"], "failed");
    assert_eq!(last["error"]["code"], "code_locked");
}

#[tokio::test(start_paused = true)]
async fn a_locked_device_fails_with_code_locked() {
    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    let id = start(&harness).await;
    harness.wait_state(POD, "awaiting_code").await;
    device.locked.store(true, Ordering::SeqCst);
    let locked = harness
        .ok(
            "/local/pod/commission/code",
            json!({ "sessionId": id, "code": CODE }),
        )
        .await;
    assert_eq!(locked["error"]["code"], "code_locked");

    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    device.locked.store(true, Ordering::SeqCst);
    start(&harness).await;
    let failed = harness.wait_state(POD, "failed").await;
    assert_eq!(failed["error"]["code"], "code_locked");
}

#[tokio::test(start_paused = true)]
async fn start_checks_account_device_and_radio() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    harness.add_device("host-a", Script::host());
    let start = |candidate: &str| json!({ "candidateId": candidate });

    *harness.cloud.account.lock().unwrap() = None;
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("pod-a"))
            .await,
        "not_signed_in"
    );
    let mut empty = support::account("ctx-1");
    empty.scopes.clear();
    *harness.cloud.account.lock().unwrap() = Some(empty);
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("pod-a"))
            .await,
        "scope_not_offered"
    );
    *harness.cloud.account.lock().unwrap() = Some(support::account("ctx-1"));

    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("gone"))
            .await,
        "device_gone"
    );
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("host-a"))
            .await,
        "wrong_kind"
    );
    assert_eq!(
        harness
            .reason("/local/host/provision/start", start("pod-a"))
            .await,
        "wrong_kind"
    );
    harness.advertise("pod-a", POD, 0, -55);
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("pod-a"))
            .await,
        "not_commissionable"
    );
    harness.advertise("pod-a", POD, BLE_FLAG_COMMISSIONABLE, -55);
    harness
        .core
        .on_adapter_state(app_facade_api::pod::AdapterStatus {
            state: app_facade_api::pod::AdapterState::PoweredOff,
            message: None,
        });
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("pod-a"))
            .await,
        "device_gone"
    );
    harness.core.on_adapter_state(support::ready());
    harness.add_device("pod-a", Script::pod());
    harness.add_device("host-a", Script::host());

    let id = session_id(
        &harness
            .ok("/local/pod/commission/start", start("pod-a"))
            .await,
    );
    assert_eq!(
        harness
            .reason("/local/pod/commission/start", start("pod-a"))
            .await,
        "session_active"
    );
    assert_eq!(
        harness
            .reason("/local/host/provision/start", start("host-a"))
            .await,
        "session_active"
    );
    assert_eq!(
        harness
            .reason("/local/host/provision/cancel", json!({ "sessionId": id }))
            .await,
        "unknown_session"
    );
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/renew",
                json!({ "sessionId": "nope" })
            )
            .await,
        "unknown_session"
    );
    let unreadable = harness
        .core
        .handle("/local/pod/commission/start", "{\"candidate\":1}")
        .await
        .unwrap_err();
    assert_eq!(unreadable.error, "BAD_REQUEST");
    let unknown = harness
        .core
        .handle("/local/pod/elsewhere", "{}")
        .await
        .unwrap_err();
    assert_eq!(unknown.error, "UNSUPPORTED");
}

#[tokio::test(start_paused = true)]
async fn requests_are_checked_against_the_session_state() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    let id = start(&harness).await;
    harness.wait_state(POD, "awaiting_code").await;
    let body = |extra: Value| {
        let mut body = json!({ "sessionId": id });
        body.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        body
    };
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/code",
                body(json!({ "code": "12345" }))
            )
            .await,
        "invalid_code"
    );
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/wifi",
                body(json!({ "ssid": "home" }))
            )
            .await,
        "wrong_state"
    );
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/authorize",
                body(json!({ "scopeId": "scope-a" }))
            )
            .await,
        "wrong_state"
    );
    harness
        .ok("/local/pod/commission/code", body(json!({ "code": CODE })))
        .await;
    harness.wait_state(POD, "awaiting_wifi").await;
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/wifi",
                body(json!({ "ssid": "home", "password": "short" }))
            )
            .await,
        "invalid_wifi"
    );
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/wifi",
                body(json!({ "ssid": "x".repeat(33) }))
            )
            .await,
        "invalid_wifi"
    );
    harness
        .ok(
            "/local/pod/commission/wifi",
            body(json!({ "ssid": "home", "password": "correct horse" })),
        )
        .await;
    harness.wait_state(POD, "awaiting_authorization").await;
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/authorize",
                body(json!({ "scopeId": "scope-z" }))
            )
            .await,
        "scope_not_offered"
    );
}

#[tokio::test(start_paused = true)]
async fn a_loopback_cloud_is_refused_before_issuing() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    let id = to_authorization(&harness).await;
    harness.cloud.endpoints.lock().unwrap().api = "http://127.0.0.1:8080/api/v1".into();
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/authorize",
                json!({ "sessionId": id, "scopeId": "scope-a" })
            )
            .await,
        "cloud_unreachable_for_device"
    );
    let session = harness.ok("/local/pod/commission/status", json!({})).await;
    assert_eq!(session["session"]["state"], "awaiting_authorization");
    harness.cloud.endpoints.lock().unwrap().api = "http://192.168.0.10:8080/api/v1".into();
    authorize(&harness, &id).await;
    harness.wait_state(POD, "completed").await;
    assert!(harness.cloud.calls.lock().unwrap()[0].path == "pod/credential/issue");
}

#[tokio::test(start_paused = true)]
async fn a_lost_link_while_activating_asks_lion() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.activation = Activation::Never;
    script.drop_after_op = Some("status");
    harness.add_device("pod-a", script);
    *harness.cloud.statuses.lock().unwrap() = ["unreachable", "pending", "pending"].into();
    *harness.cloud.status.lock().unwrap() = "active";
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    let done = harness
        .wait(POD, false, |session| session["state"] == "completed")
        .await;
    assert_eq!(done["podId"], "pod-1");
    assert!(harness.cloud.revokes().is_empty());
    let statuses = harness
        .cloud
        .paths()
        .iter()
        .filter(|path| *path == "pod/credential/status")
        .count();
    assert_eq!(statuses, 4);
}

#[tokio::test(start_paused = true)]
async fn a_lost_delivery_that_lion_never_saw_fails_without_revoking() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.drop_after_op = Some("deliver");
    harness.add_device("pod-a", script);
    *harness.cloud.status.lock().unwrap() = "absent";
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    let failed = harness.wait_state(POD, "failed").await;
    assert_eq!(failed["error"]["code"], "activation_failed");
    assert_eq!(failed["error"]["detail"], "not_activated");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(harness.cloud.revokes().is_empty());
}

#[tokio::test(start_paused = true)]
async fn a_credential_still_pending_after_its_window_is_revoked() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.drop_after_op = Some("deliver");
    harness.add_device("pod-a", script);
    let id = to_authorization(&harness).await;
    let issued_at = tokio::time::Instant::now();
    authorize(&harness, &id).await;
    let failed = harness
        .wait(POD, false, |session| session["state"] == "failed")
        .await;
    assert!(issued_at.elapsed() >= Duration::from_secs(630));
    assert_eq!(failed["error"]["code"], "activation_failed");
    assert_eq!(failed["error"]["detail"], "not_activated");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let revokes = harness.cloud.revokes();
    assert_eq!(revokes.len(), 1);
    assert_eq!(revokes[0].body["podId"], "pod-1");
    assert_eq!(revokes[0].context, "ctx-1");
}

#[tokio::test(start_paused = true)]
async fn a_pod_activation_failure_revokes_with_the_issuing_account() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.activation = Activation::Failed("time_sync_failed");
    harness.add_device("pod-a", script);
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    *harness.cloud.account.lock().unwrap() = Some(support::account("ctx-2"));
    let failed = harness.wait_state(POD, "failed").await;
    assert_eq!(failed["error"]["code"], "activation_failed");
    assert_eq!(failed["error"]["detail"], "time_sync_failed");
    tokio::time::sleep(Duration::from_secs(1)).await;
    let revokes = harness.cloud.revokes();
    assert_eq!(revokes.len(), 1);
    assert_eq!(revokes[0].context, "ctx-1");
}

#[tokio::test(start_paused = true)]
async fn a_refused_delivery_revokes() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.deliver_ok = false;
    harness.add_device("pod-a", script);
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    let failed = harness.wait_state(POD, "failed").await;
    assert_eq!(failed["error"]["code"], "delivery_failed");
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert_eq!(harness.cloud.revokes().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn an_unrenewed_session_is_cancelled_but_activation_runs_on() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    start(&harness).await;
    let cancelled = harness
        .wait(POD, false, |session| session["state"] == "cancelled")
        .await;
    assert!(cancelled.get("error").is_none());

    let harness = Harness::new();
    let mut script = Script::pod();
    script.activation = Activation::Commissioned { after_polls: 50 };
    harness.add_device("pod-a", script);
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    let done = harness
        .wait(POD, false, |session| {
            matches!(session["state"].as_str(), Some("completed" | "cancelled"))
        })
        .await;
    assert_eq!(done["state"], "completed");
}

#[tokio::test(start_paused = true)]
async fn cancelling_while_issuing_revokes_the_late_credential() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    *harness.cloud.issue_delay.lock().unwrap() = Duration::from_secs(3);
    let id = to_authorization(&harness).await;
    let (issued, cancelled) = tokio::join!(authorize(&harness, &id), async {
        tokio::time::sleep(Duration::from_secs(1)).await;
        harness
            .ok("/local/pod/commission/cancel", json!({ "sessionId": id }))
            .await
    });
    assert_eq!(cancelled["state"], "cancelled");
    assert_eq!(issued["state"], "cancelled");
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(harness.cloud.revokes().len(), 1);
    let status = harness.ok("/local/pod/commission/status", json!({})).await;
    assert_eq!(status["session"]["state"], "cancelled");
}

#[tokio::test(start_paused = true)]
async fn cancelling_while_activating_revokes() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.activation = Activation::Never;
    harness.add_device("pod-a", script);
    let id = to_authorization(&harness).await;
    authorize(&harness, &id).await;
    harness.wait_state(POD, "activating").await;
    let cancelled = harness
        .ok("/local/pod/commission/cancel", json!({ "sessionId": id }))
        .await;
    assert_eq!(cancelled["state"], "cancelled");
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(harness.cloud.revokes().len(), 1);
    let again = harness
        .ok("/local/pod/commission/cancel", json!({ "sessionId": id }))
        .await;
    assert_eq!(again["revision"], cancelled["revision"]);
}

#[tokio::test(start_paused = true)]
async fn a_kept_network_is_confirmed_before_skipping_wifi() {
    let harness = Harness::new();
    let mut script = Script::pod();
    script.wifi_configured = true;
    script.kept_wifi_connecting_polls = 4;
    let device = harness.add_device("pod-a", script);
    let id = start(&harness).await;
    secure(&harness, &id).await;
    harness.wait_state(POD, "awaiting_authorization").await;
    assert_eq!(device.scans.load(Ordering::SeqCst), 0);

    let harness = Harness::new();
    let mut script = Script::pod();
    script.wifi_configured = true;
    script.kept_wifi =
        support::Join::Failed(protocomm::proto::WifiConnectFailedReason::NetworkNotFound);
    harness.add_device("pod-a", script);
    let id = start(&harness).await;
    secure(&harness, &id).await;
    let offered = harness.wait_state(POD, "awaiting_wifi").await;
    assert_eq!(offered["error"]["code"], "wifi_not_found");
    harness
        .ok(
            "/local/pod/commission/wifi",
            json!({ "sessionId": id, "ssid": "home", "password": "correct horse" }),
        )
        .await;
    harness.wait_state(POD, "awaiting_authorization").await;
    assert!(harness.position("device:reset") < harness.position("device:set_config"));
}

#[tokio::test(start_paused = true)]
async fn a_scan_request_during_the_first_scan_waits_for_it() {
    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    let id = start(&harness).await;
    secure(&harness, &id).await;
    harness.wait_state(POD, "awaiting_wifi").await;
    let session = harness
        .ok(
            "/local/pod/commission/wifi/scan",
            json!({ "sessionId": id }),
        )
        .await;
    assert_eq!(session["networks"][0]["ssid"], "home");
    assert!(device.scans.load(Ordering::SeqCst) <= 2);
}

#[tokio::test(start_paused = true)]
async fn the_session_times_out_after_its_lifetime() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    start(&harness).await;
    let timed_out = harness.wait_state(POD, "timed_out").await;
    assert_eq!(timed_out["error"]["code"], "session_timeout");
}

#[tokio::test(start_paused = true)]
async fn a_dropped_link_while_waiting_fails_the_session() {
    let harness = Harness::new();
    let device = harness.add_device("pod-a", Script::pod());
    start(&harness).await;
    harness.wait_state(POD, "awaiting_code").await;
    device.drop_link();
    let failed = harness.wait_state(POD, "failed").await;
    assert_eq!(failed["error"]["code"], "disconnected");
}

#[tokio::test(start_paused = true)]
async fn a_refused_issue_fails_without_a_credential_to_revoke() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    harness.cloud.fail_issue.store(true, Ordering::SeqCst);
    let id = to_authorization(&harness).await;
    let failed = authorize(&harness, &id).await;
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["error"]["code"], "issue_failed");
    valid(&failed);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(harness.cloud.revokes().is_empty());
    assert!(!harness
        .journal()
        .iter()
        .any(|entry| entry.starts_with("device:pod-credential")));
}
