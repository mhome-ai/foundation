mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use app_facade_api::host_provision::{ProvisionSession, PROVISION_CHANGED_EVENT};
use app_facade_api::pod::DeviceKind;
use protocomm::proto::WifiConnectFailedReason;
use serde_json::{json, Value};
use support::{session_id, Harness, Join, Script, CLAIM_TOKEN, CODE};

const HOST: DeviceKind = DeviceKind::Host;

async fn start(harness: &Harness) -> String {
    let session = harness
        .ok(
            "/local/host/provision/start",
            json!({ "candidateId": "host-a" }),
        )
        .await;
    assert_eq!(session["state"], "connecting");
    session_id(&session)
}

async fn to_wifi(harness: &Harness) -> String {
    let id = start(harness).await;
    harness.wait_state(HOST, "awaiting_code").await;
    harness
        .ok(
            "/local/host/provision/code",
            json!({ "sessionId": id, "code": CODE }),
        )
        .await;
    let session = harness
        .wait(HOST, true, |session| {
            session["state"] == "awaiting_wifi" && session["networks"].is_array()
        })
        .await;
    assert_eq!(session["host"]["hostId"], "host-1");
    id
}

async fn join(harness: &Harness, id: &str) -> Value {
    harness
        .ok(
            "/local/host/provision/wifi",
            json!({ "sessionId": id, "ssid": "home", "password": "correct horse" }),
        )
        .await
}

fn valid(session: &Value) {
    let parsed: ProvisionSession = serde_json::from_value(session.clone()).unwrap();
    parsed.validate().unwrap();
}

#[tokio::test(start_paused = true)]
async fn provisions_a_host_and_hands_back_its_claim_token() {
    let harness = Harness::new();
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["networks"].as_array().unwrap().len(), 7);
    assert_eq!(done["addresses"], json!(["192.168.0.50"]));
    assert_eq!(done["claimToken"], CLAIM_TOKEN);
    valid(&done);
    assert!(harness.position("device:host-info:finish") < harness.position("event:completed"));
    assert!(harness.cloud.calls.lock().unwrap().is_empty());
    harness
        .events(PROVISION_CHANGED_EVENT)
        .iter()
        .for_each(valid);
}

#[tokio::test(start_paused = true)]
async fn a_host_without_a_claim_token_still_completes() {
    let harness = Harness::new();
    let mut script = Script::host();
    script.claim_token = None;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert!(done.get("claimToken").is_none());
}

#[tokio::test(start_paused = true)]
async fn a_host_locks_after_repeated_wrong_codes() {
    let harness = Harness::new();
    let device = harness.add_device("host-a", Script::host());
    let id = start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;
    let rejected = harness
        .ok(
            "/local/host/provision/code",
            json!({ "sessionId": id, "code": "000000" }),
        )
        .await;
    assert_eq!(rejected["state"], "awaiting_code");
    assert_eq!(rejected["error"]["code"], "code_rejected");
    let mut last = rejected;
    for _ in 0..4 {
        last = harness
            .ok(
                "/local/host/provision/code",
                json!({ "sessionId": id, "code": "000000" }),
            )
            .await;
    }
    assert!(device.locked.load(Ordering::SeqCst));
    assert_eq!(last["state"], "failed");
    assert_eq!(last["error"]["code"], "code_locked");
}

#[tokio::test(start_paused = true)]
async fn a_failed_join_offers_wifi_again_and_resets_first() {
    let harness = Harness::new();
    let mut script = Script::host();
    script.join = Join::Failed(WifiConnectFailedReason::AuthError);
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let offered = harness
        .wait(HOST, true, |session| {
            session["state"] == "awaiting_wifi" && session.get("error").is_some()
        })
        .await;
    assert_eq!(offered["error"]["code"], "wifi_auth_failed");
    valid(&offered);

    device.script.lock().unwrap().join = Join::Connected("10.0.0.7");
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["addresses"], json!(["10.0.0.7"]));
    let journal = harness.journal();
    let second_config = journal
        .iter()
        .rposition(|entry| entry == "device:set_config")
        .unwrap();
    let reset = journal
        .iter()
        .rposition(|entry| entry == "device:reset")
        .unwrap();
    assert!(reset < second_config);
}

#[tokio::test(start_paused = true)]
async fn a_join_that_never_settles_fails_as_wifi_failed() {
    let harness = Harness::new();
    let mut script = Script::host();
    script.join = Join::Never;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let offered = harness
        .wait(HOST, true, |session| session.get("error").is_some())
        .await;
    assert_eq!(offered["state"], "awaiting_wifi");
    assert_eq!(offered["error"]["code"], "wifi_failed");
}

#[tokio::test(start_paused = true)]
async fn pod_and_host_sessions_are_kept_apart() {
    let harness = Harness::new();
    harness.add_device("pod-a", Script::pod());
    harness.add_device("host-a", Script::host());
    let pod = session_id(
        &harness
            .ok(
                "/local/pod/commission/start",
                json!({ "candidateId": "pod-a" }),
            )
            .await,
    );
    harness
        .ok("/local/pod/commission/cancel", json!({ "sessionId": pod }))
        .await;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let host = start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;

    let pod_status = harness.ok("/local/pod/commission/status", json!({})).await;
    assert_eq!(pod_status["session"]["sessionId"], pod);
    assert_eq!(pod_status["session"]["state"], "cancelled");
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/code",
                json!({ "sessionId": host, "code": CODE })
            )
            .await,
        "unknown_session"
    );
    assert_eq!(
        harness
            .reason(
                "/local/pod/commission/start",
                json!({ "candidateId": "pod-a" })
            )
            .await,
        "session_active"
    );
}

#[tokio::test(start_paused = true)]
async fn host_lease_expiry_cancels() {
    let harness = Harness::new();
    harness.add_device("host-a", Script::host());
    start(&harness).await;
    let cancelled = harness
        .wait(HOST, false, |session| session["state"] == "cancelled")
        .await;
    valid(&cancelled);
    assert!(harness
        .journal()
        .contains(&"device:disconnected".to_string()));
}
