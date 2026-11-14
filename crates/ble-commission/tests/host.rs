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

async fn complete(harness: &Harness) -> Value {
    let session = harness
        .wait(HOST, |s| {
            s["state"] == "completed" || s["state"] == "awaiting_authorization"
        })
        .await;
    if session["state"] == "awaiting_authorization" {
        assert!(session.get("claimToken").is_none());
        harness
            .ok(
                "/local/host/provision/authorize",
                json!({"sessionId":session["sessionId"]}),
            )
            .await;
    }
    harness.wait_state(HOST, "completed").await
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
    let done = complete(&harness).await;
    assert!(done.get("claimToken").is_none());
    assert_eq!(done["result"]["commissioned"], true);
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
    let done = complete(&harness).await;
    assert!(done.get("claimToken").is_none());
    assert_eq!(done["result"]["commissioned"], true);
    assert!(harness.position("device:reset") > harness.position("device:apply"));
}

#[tokio::test(start_paused = true)]
async fn provisions_a_host_and_returns_its_completed_claim_result() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = complete(&harness).await;
    assert_eq!(done["networks"].as_array().unwrap().len(), 7);
    assert_eq!(done["addresses"], json!(["192.168.0.50"]));
    assert!(done.get("claimToken").is_none());
    assert_eq!(done["result"]["commissioned"], true);
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
async fn space_host_requires_explicit_space_and_prepares_its_exact_hub_before_wifi() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    let status = harness.ok("/local/host/provision/status", json!({})).await;
    assert!(status["session"]["authorization"]["spaces"].is_array());
    for enrollment in [
        Value::Null,
        json!({"spaceId":"scope-a","clientMode":"user"}),
        json!({"spaceId":"scope-b","clientMode":"space"}),
    ] {
        assert!(harness
            .call(
                "/local/host/provision/wifi",
                json!({"sessionId":id,"ssid":"home","enrollment":enrollment})
            )
            .await
            .is_err());
    }
    assert!(!harness
        .journal()
        .iter()
        .any(|entry| entry == "device:apply"));
    harness.ok("/local/host/provision/wifi", json!({"sessionId":id,"ssid":"home","enrollment":{"spaceId":"scope-a","clientMode":"space","firstHub":false}})).await;
    let done = complete(&harness).await;
    valid(&done);
    let targets = device.network_requests.lock().unwrap();
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0]["mode"], "space");
    assert_eq!(
        &targets[0]["hub"].as_array().unwrap()[..3],
        &[json!("hub"), json!("tenant"), json!("scope-a")]
    );
    assert!(harness.position("device:host-info:prepareWifi") < harness.position("device:apply"));
}

#[tokio::test(start_paused = true)]
async fn space_host_bootstrap_requires_the_explicit_first_hub_intent() {
    for first_hub in [false, true] {
        let harness = Harness::with_discovery("ble").await;
        harness.cloud.bootstrap.store(true, Ordering::SeqCst);
        let mut script = Script::host();
        script.host_mode = core_api::host::access::HostMode::SpaceSelf;
        let device = harness.add_device("host-a", script);
        let id = to_wifi(&harness).await;
        harness.ok("/local/host/provision/wifi", json!({"sessionId":id,"ssid":"home","enrollment":{"spaceId":"scope-a","clientMode":"space","firstHub":first_hub}})).await;
        let done = if first_hub {
            complete(&harness).await
        } else {
            harness.wait_state(HOST, "failed").await
        };
        valid(&done);
        if first_hub {
            assert_eq!(
                *device.network_requests.lock().unwrap(),
                vec![
                    json!({"op":"prepareWifi","mode":"bootstrap","firstHub":{"intentId":CLAIM_TOKEN,"space":{"tenantId":"tenant","spaceId":"scope-a"}}})
                ]
            );
        } else {
            assert!(device.network_requests.lock().unwrap().is_empty());
            assert!(!harness
                .journal()
                .iter()
                .any(|entry| entry == "device:apply"));
        }
    }
}

#[tokio::test(start_paused = true)]
async fn existing_own_hub_changes_only_wifi_without_looking_up_a_remote_hub() {
    let harness = Harness::with_discovery("ble").await;
    harness.cloud.bootstrap.store(true, Ordering::SeqCst);
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    script.onboarding_required = false;
    script.claim_token = None;
    script.own_product_hub = true;
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = complete(&harness).await;
    valid(&done);
    assert!(done.get("claimToken").is_none());
    assert!(harness.cloud.calls.lock().unwrap().is_empty());
    assert_eq!(
        *device.network_requests.lock().unwrap(),
        vec![json!({"op":"prepareWifi","mode":"ownHub"})]
    );
}

#[tokio::test(start_paused = true)]
async fn ordinary_space_host_cannot_reprovision_without_an_available_hub() {
    let harness = Harness::with_discovery("ble").await;
    harness.cloud.bootstrap.store(true, Ordering::SeqCst);
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    script.onboarding_required = false;
    script.claim_token = None;
    let device = harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    valid(&harness.wait_state(HOST, "failed").await);
    assert!(device.network_requests.lock().unwrap().is_empty());
    assert!(!harness
        .journal()
        .iter()
        .any(|entry| entry == "device:apply"));
}

#[tokio::test(start_paused = true)]
async fn a_host_without_a_claim_token_still_completes() {
    let harness = Harness::with_discovery("ble").await;
    let mut script = Script::host();
    script.onboarding_required = false;
    script.claim_token = None;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let done = complete(&harness).await;
    assert!(done.get("claimToken").is_none());
}

#[tokio::test(start_paused = true)]
async fn host_does_not_complete_without_the_expected_finish_receipt() {
    for (needs_claim, refused, token) in [
        (true, true, Some(CLAIM_TOKEN)),
        (true, false, None),
        (true, false, Some("bad")),
        (false, false, Some(CLAIM_TOKEN)),
    ] {
        let harness = Harness::with_discovery("ble").await;
        let mut script = Script::host();
        script.onboarding_required = needs_claim;
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
    let done = complete(&harness).await;
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
    let done = complete(&harness).await;
    assert!(done.get("claimToken").is_none());
    assert_eq!(done["result"]["commissioned"], true);
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
    complete(&harness).await;
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

#[tokio::test(start_paused = true)]
async fn authorization_requires_explicit_confirmation_and_never_exposes_the_token() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    let waiting = harness.wait_state(HOST, "awaiting_authorization").await;
    valid(&waiting);
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert!(!harness.journal().iter().any(|s| s == "host:claim"));
    assert!(waiting.get("claimToken").is_none());
    let done = complete(&harness).await;
    assert_eq!(
        done["result"],
        json!({"hostId":"host-1","wifiConfigured":true,"commissioned":true})
    );
    assert!(harness.position("device:disconnected") < harness.position("host:claim"));
    for event in harness.events(PROVISION_CHANGED_EVENT) {
        assert!(event.get("claimToken").is_none());
        valid(&event);
    }
}

#[tokio::test(start_paused = true)]
async fn a_failed_claim_requires_explicit_retry_without_repeating_wifi() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    harness.cloud.fail_host_claim.store(true, Ordering::SeqCst);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    harness.wait_state(HOST, "awaiting_authorization").await;
    harness
        .ok("/local/host/provision/authorize", json!({"sessionId":id}))
        .await;
    let failed = harness
        .wait(HOST, |s| s["error"]["code"] == "issue_failed")
        .await;
    assert_eq!(failed["state"], "awaiting_authorization");
    valid(&failed);
    tokio::time::sleep(Duration::from_secs(5)).await;
    assert_eq!(
        harness
            .journal()
            .iter()
            .filter(|s| *s == "host:claim")
            .count(),
        1
    );
    harness.cloud.fail_host_claim.store(false, Ordering::SeqCst);
    complete(&harness).await;
    assert_eq!(
        harness
            .journal()
            .iter()
            .filter(|s| *s == "host:claim")
            .count(),
        2
    );
    assert_eq!(
        harness
            .journal()
            .iter()
            .filter(|s| *s == "device:apply")
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn cancellation_and_repeated_authorize_cannot_interrupt_a_claim() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    *harness.cloud.host_claim_delay.lock().unwrap() = Duration::from_secs(10);
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    harness.wait_state(HOST, "awaiting_authorization").await;
    harness
        .ok("/local/host/provision/authorize", json!({"sessionId":id}))
        .await;
    harness.wait_state(HOST, "authorizing").await;
    assert_eq!(
        harness
            .reason("/local/host/provision/cancel", json!({"sessionId":id}))
            .await,
        "busy"
    );
    assert!(harness
        .call("/local/host/provision/authorize", json!({"sessionId":id}))
        .await
        .is_err());
    harness.wait_state(HOST, "completed").await;
    assert_eq!(
        harness
            .journal()
            .iter()
            .filter(|s| *s == "host:claim")
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn account_switch_retires_a_pending_claim_instead_of_transferring_it() {
    let harness = Harness::with_discovery("ble").await;
    harness.add_device("host-a", Script::host());
    let id = to_wifi(&harness).await;
    join(&harness, &id).await;
    harness.wait_state(HOST, "awaiting_authorization").await;
    *harness.cloud.account.lock().unwrap() = Some(support::account("ctx-2"));
    assert!(harness
        .call("/local/host/provision/authorize", json!({"sessionId":id}))
        .await
        .is_err());
    assert!(harness.ok("/local/host/provision/status", json!({})).await["session"].is_null());
    assert!(!harness.journal().iter().any(|s| s == "host:claim"));
}

#[tokio::test(start_paused = true)]
async fn first_hub_failure_preserves_the_paired_claim_for_explicit_recovery() {
    let harness = Harness::with_discovery("ble").await;
    harness.cloud.bootstrap.store(true, Ordering::SeqCst);
    harness.cloud.fail_hub_setup.store(true, Ordering::SeqCst);
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    harness.add_device("host-a", script);
    let id = to_wifi(&harness).await;
    harness.ok("/local/host/provision/wifi", json!({"sessionId":id,"ssid":"home","enrollment":{"spaceId":"scope-a","clientMode":"space","firstHub":true}})).await;
    harness.wait_state(HOST, "awaiting_authorization").await;
    harness
        .ok("/local/host/provision/authorize", json!({"sessionId":id}))
        .await;
    let failed = harness
        .wait(HOST, |s| s["error"]["code"] == "issue_failed")
        .await;
    valid(&failed);
    assert!(harness.journal().iter().any(|s| s == "host:claimed"));
    assert!(failed.get("result").is_none());
    harness.cloud.fail_hub_setup.store(false, Ordering::SeqCst);
    let done = complete(&harness).await;
    assert_eq!(done["result"]["hubId"], "hub");
    assert!(harness.position("host:claimed") < harness.position("product:hub-ready"));
    assert_eq!(
        harness
            .journal()
            .iter()
            .filter(|s| *s == "device:apply")
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn claimed_first_hub_resumes_after_wifi_without_reclaiming_or_exposing_tokens() {
    let h = Harness::with_discovery("ble").await;
    h.cloud.bootstrap.store(true, Ordering::SeqCst);
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    script.onboarding_required = false;
    script.claim_token = None;
    script.first_hub_recovery = true;
    let device = h.add_device("host-a", script);
    let id = to_wifi(&h).await;
    join(&h, &id).await;
    let ready = h.wait_state(HOST, "awaiting_authorization").await;
    assert!(ready.get("claimToken").is_none());
    assert!(ready.get("result").is_none());
    assert_eq!(ready["enrollment"]["firstHub"], true);
    let done = complete(&h).await;
    valid(&done);
    assert_eq!(done["result"]["hubId"], "hub");
    let commands = device.network_requests.lock().unwrap();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0]["mode"], "bootstrapRecovery");
    assert_eq!(commands[0]["firstHub"]["space"]["spaceId"], "scope-a");
    assert!(serde_json::to_vec(&commands[0]).unwrap().len() < 512);
    assert!(!h.journal().iter().any(|s| s == "host:claim"));
    assert!(h.position("cloud:/host/first-hub/recovery/prepare") < h.position("device:apply"));
}
#[tokio::test(start_paused = true)]
async fn closed_first_hub_intent_fails_before_transmitting_wifi_credentials() {
    let h = Harness::with_discovery("ble").await;
    h.cloud.bootstrap.store(true, Ordering::SeqCst);
    h.cloud.fail_first_hub_network.store(true, Ordering::SeqCst);
    let mut script = Script::host();
    script.host_mode = core_api::host::access::HostMode::SpaceSelf;
    script.onboarding_required = false;
    script.claim_token = None;
    script.first_hub_recovery = true;
    let device = h.add_device("host-a", script);
    let id = to_wifi(&h).await;
    join(&h, &id).await;
    let failed = h.wait_state(HOST, "failed").await;
    valid(&failed);
    assert_eq!(failed["error"]["code"], "wifi_failed");
    assert!(device.network_requests.lock().unwrap().is_empty());
    assert!(!h.journal().iter().any(|s| s == "device:set_config"));
}
