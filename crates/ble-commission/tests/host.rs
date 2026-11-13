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
        .wait(HOST, |session| {
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
async fn wifi_submission_reserves_the_critical_section_before_the_device_reply() {
    let harness = Harness::with_discovery("ble").await;
    let device = harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    *device.response_delay.lock().unwrap() = Duration::from_secs(2);
    let cancellation = async {
        loop {
            if harness
                .journal()
                .iter()
                .any(|entry| entry == "device:apply")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(
            harness
                .reason("/local/host/provision/cancel", json!({"sessionId": id}))
                .await,
            "busy"
        );
        let status = harness.ok("/local/host/provision/status", json!({})).await;
        assert_eq!(status["session"]["state"], "joining_wifi");
    };
    tokio::join!(join(&harness, &id), cancellation);
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["claimToken"], CLAIM_TOKEN);
}

#[tokio::test(start_paused = true)]
async fn a_rejected_wifi_apply_releases_the_critical_section_for_an_explicit_retry() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.reject_wifi_apply = true;
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    let rejected = join(&harness, &id).await;
    assert_eq!(rejected["state"], "awaiting_wifi");
    assert_eq!(rejected["error"]["code"], "wifi_failed");
    valid(&rejected);
    device.script.lock().unwrap().reject_wifi_apply = false;
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["claimToken"], CLAIM_TOKEN);
    assert!(harness.position("device:reset") > harness.position("device:apply"));
}

#[tokio::test(start_paused = true)]
async fn provisions_a_host_and_hands_back_its_claim_token() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["networks"].as_array().unwrap().len(), 7);
    assert_eq!(done["addresses"], json!(["192.168.0.50"]));
    assert_eq!(done["claimToken"], CLAIM_TOKEN);
    valid(&done);
    harness
        .wait_event(PROVISION_CHANGED_EVENT, |event| {
            event["sessionId"] == id && event["state"] == "completed"
        })
        .await;
    assert!(harness.position("device:host-info:finish") < harness.position("event:completed"));
    assert!(harness.cloud.calls.lock().unwrap().is_empty());
    harness
        .events(PROVISION_CHANGED_EVENT)
        .iter()
        .for_each(valid);
}

#[tokio::test(start_paused = true)]
async fn a_host_without_a_claim_token_still_completes() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.claim_token = None;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = harness.wait_state(HOST, "completed").await;
    assert!(done.get("claimToken").is_none());
}

#[tokio::test(start_paused = true)]
async fn host_does_not_complete_without_the_expected_finish_receipt() {
    for (refused, token) in [
        (true, Some(CLAIM_TOKEN)),
        (false, None),
        (false, Some("bad")),
    ] {
        let harness = Harness::with_discovery("ble").await;
        let mut script = Script::host();
        script.onboarding_required = true;
        script.refuse_host_finish = refused;
        script.claim_token = token;
        harness.add_device("host-a", script);
        let id = to_wifi(&harness).await;
        join(&harness, &id).await;
        let session = harness.wait_state(HOST, "failed").await;
        assert_eq!(session["error"]["code"], "wifi_failed");
        assert!(session.get("claimToken").is_none());
        valid(&session);
    }
}

#[tokio::test(start_paused = true)]
async fn a_host_locks_after_repeated_wrong_codes() {
    let harness = Harness::with_discovery("ble").await;
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
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.join = Join::Failed(WifiConnectFailedReason::AuthError);
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let offered = harness
        .wait(HOST, |session| {
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
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.join = Join::Never;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let offered = harness
        .wait(HOST, |session| session.get("error").is_some())
        .await;
    assert_eq!(offered["state"], "awaiting_wifi");
    assert_eq!(offered["error"]["code"], "wifi_failed");
}

#[tokio::test(start_paused = true)]
async fn pod_and_host_sessions_are_kept_apart() {
    let harness = Harness::with_discovery("ble").await;
    let device = harness.add_device("pod-a", Script::pod());
    harness.add_device("host-a", Script::host());
    let pod = session_id(
        &harness
            .ok(
                "/local/pod/commission/start",
                json!({ "candidateId": "pod-a" }),
            )
            .await,
    );
    harness.wait_state(DeviceKind::Pod, "awaiting_code").await;
    device.drop_link();
    tokio::time::sleep(Duration::from_secs(1)).await;
    let host = start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;

    let pod_status = harness.ok("/local/pod/commission/status", json!({})).await;
    assert_eq!(pod_status["session"]["sessionId"], pod);
    assert_eq!(pod_status["session"]["state"], "failed");
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
async fn host_without_ui_expires_only_at_its_deadline() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    start(&harness).await;
    let timed_out = harness
        .wait(HOST, |session| session["state"] == "timed_out")
        .await;
    valid(&timed_out);
    tokio::time::sleep(Duration::from_secs(1)).await;
    assert!(harness
        .journal()
        .contains(&"device:disconnected".to_string()));
}

#[tokio::test(start_paused = true)]
async fn cancelling_releases_the_link_before_its_terminal_event() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    let cancelling = harness
        .ok("/local/host/provision/cancel", json!({"sessionId": id}))
        .await;
    assert_eq!(cancelling["state"], "cancelling");
    let cancelled = harness.wait_state(HOST, "cancelled").await;
    valid(&cancelled);
    harness
        .wait_event(PROVISION_CHANGED_EVENT, |event| {
            event["sessionId"] == id && event["state"] == "cancelled"
        })
        .await;
    assert!(harness.position("device:disconnected") < harness.position("event:cancelled"));
    // A new operation can acquire the radio immediately after the terminal event.
    start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;
}

#[tokio::test(start_paused = true)]
async fn cancellation_during_connect_closes_the_late_link_without_reopening_setup() {
    let harness = Harness::with_discovery("ble").await;
    let device = harness.add_device("host-a", Script::host());
    let id = start(&harness).await;
    // The fake central takes 50 ms to connect. Cancel while it still owns that work.
    tokio::time::sleep(Duration::from_millis(10)).await;
    let response = harness
        .ok("/local/host/provision/cancel", json!({"sessionId": id}))
        .await;
    assert_eq!(response["state"], "cancelling");
    valid(&harness.wait_state(HOST, "cancelled").await);
    harness
        .wait_event(PROVISION_CHANGED_EVENT, |event| {
            event["sessionId"] == id && event["state"] == "cancelled"
        })
        .await;
    assert!(!device.connected.load(Ordering::SeqCst));
    assert!(harness.position("device:disconnected") < harness.position("event:cancelled"));
    assert!(!harness
        .events(PROVISION_CHANGED_EVENT)
        .iter()
        .any(|event| { event["sessionId"] == id && event["state"] == "awaiting_code" }));
    start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;
}

#[tokio::test(start_paused = true)]
async fn cancellation_cannot_interrupt_the_network_and_token_handoff() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.join = Join::Never;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    assert_eq!(
        harness
            .reason("/local/host/provision/cancel", json!({"sessionId":id}))
            .await,
        "busy"
    );
}

#[tokio::test(start_paused = true)]
async fn unclaimed_host_with_a_live_saved_network_recovers_without_wifi_input() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.wifi_configured = true;
    script.onboarding_required = true;
    script.network_online = true;
    harness.add_device("host-a", script);
    let id = start(&harness).await;
    harness.wait_state(HOST, "awaiting_code").await;
    harness
        .ok(
            "/local/host/provision/code",
            json!({"sessionId":id, "code":CODE}),
        )
        .await;
    let done = harness.wait_state(HOST, "completed").await;
    assert_eq!(done["claimToken"], CLAIM_TOKEN);
    assert!(!harness
        .journal()
        .iter()
        .any(|entry| entry == "device:set_config"));
}

#[tokio::test(start_paused = true)]
async fn an_offline_saved_network_still_offers_wifi() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.wifi_configured = true;
    script.onboarding_required = true;
    harness.add_device("host-a", script);
    to_wifi(&harness).await;
}

#[tokio::test(start_paused = true)]
async fn a_completed_handoff_is_not_exposed_to_another_account() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    harness.wait_state(HOST, "completed").await;
    *harness.cloud.account.lock().unwrap() = None;
    let status = harness.ok("/local/host/provision/status", json!({})).await;
    assert!(status["session"].is_null());
    assert_eq!(
        harness
            .reason(
                "/local/host/provision/code",
                json!({"sessionId":id, "code":CODE})
            )
            .await,
        "not_signed_in"
    );
}

#[tokio::test(start_paused = true)]
async fn logout_still_notifies_native_keepalive_after_releasing_the_old_session() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    *harness.cloud.account.lock().unwrap() = None;
    harness
        .wait_event(PROVISION_CHANGED_EVENT, |event| {
            event["sessionId"] == id && event["state"] == "cancelled"
        })
        .await;
    assert!(harness.journal().contains(&"device:disconnected".into()));
}
