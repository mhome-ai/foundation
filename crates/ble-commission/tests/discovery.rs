mod support;

use std::sync::atomic::Ordering;
use std::time::Duration;

use app_facade_api::pod::{
    AdapterState, AdapterStatus, DeviceKind, DiscoverySnapshot, BLE_FLAG_COMMISSIONABLE,
    DISCOVERY_CHANGED_EVENT,
};
use serde_json::{json, Value};
use support::{ready, Harness, Script};

fn scans(harness: &Harness) -> Vec<bool> {
    harness.radio.calls.lock().unwrap().clone()
}

async fn lease(harness: &Harness) -> String {
    let lease = harness.ok("/local/pod/discovery/start", json!({})).await;
    assert!(lease["expiresAtMs"].as_i64().unwrap() > 0);
    lease["leaseId"].as_str().unwrap().to_string()
}

async fn settle() {
    tokio::time::sleep(Duration::from_millis(100)).await;
}

#[tokio::test(start_paused = true)]
async fn a_lease_scans_until_it_is_stopped() {
    let harness = Harness::new();
    settle().await;
    assert!(scans(&harness).is_empty());
    let id = lease(&harness).await;
    settle().await;
    assert_eq!(scans(&harness), [true]);
    harness.add_device("pod-a", Script::pod());
    let list: DiscoverySnapshot =
        serde_json::from_value(harness.ok("/local/pod/discovery/list", json!({})).await).unwrap();
    assert!(list.scanning);
    assert_eq!(list.candidates.len(), 1);
    assert_eq!(list.candidates[0].candidate_id, "pod-a");
    assert_eq!(list.candidates[0].kind, DeviceKind::Pod);
    assert!(list.candidates[0].commissionable);

    harness
        .ok("/local/pod/discovery/stop", json!({ "leaseId": id }))
        .await;
    settle().await;
    assert_eq!(scans(&harness), [true, false]);
}

#[tokio::test(start_paused = true)]
async fn an_unrenewed_lease_stops_scanning_and_stale_candidates_go() {
    let harness = Harness::new();
    let id = lease(&harness).await;
    harness.add_device("pod-a", Script::pod());
    tokio::time::sleep(Duration::from_secs(12)).await;
    let list = harness.ok("/local/pod/discovery/list", json!({})).await;
    assert_eq!(list["candidates"], json!([]));
    harness
        .ok("/local/pod/discovery/renew", json!({ "leaseId": id }))
        .await;
    tokio::time::sleep(Duration::from_secs(29)).await;
    assert_eq!(scans(&harness), [true]);
    tokio::time::sleep(Duration::from_secs(3)).await;
    assert_eq!(scans(&harness), [true, false]);
}

#[tokio::test(start_paused = true)]
async fn discovery_events_are_coalesced_to_one_per_second() {
    let harness = Harness::new();
    lease(&harness).await;
    for step in 0..100 {
        let rssi = if step % 2 == 0 { -40 } else { -80 };
        harness.advertise("pod-a", DeviceKind::Pod, BLE_FLAG_COMMISSIONABLE, rssi);
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    let events = harness.events(DISCOVERY_CHANGED_EVENT);
    assert!(events.len() >= 2, "{} events", events.len());
    assert!(events.len() <= 8, "{} events", events.len());
    let last: DiscoverySnapshot = serde_json::from_value(events.last().unwrap().clone()).unwrap();
    assert_eq!(last.candidates[0].rssi, -80);
    let revisions: Vec<u64> = events
        .iter()
        .map(|event| event["revision"].as_u64().unwrap())
        .collect();
    assert!(revisions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[tokio::test(start_paused = true)]
async fn a_session_pauses_scanning_until_it_is_released() {
    let harness = Harness::new();
    let lease_id = lease(&harness).await;
    let device = harness.add_device("pod-a", Script::pod());
    settle().await;
    harness
        .ok(
            "/local/pod/commission/start",
            json!({ "candidateId": "pod-a" }),
        )
        .await;
    harness
        .ok("/local/pod/discovery/renew", json!({ "leaseId": lease_id }))
        .await;
    settle().await;
    assert_eq!(scans(&harness), [true, false]);
    harness.wait_state(DeviceKind::Pod, "awaiting_code").await;
    device.drop_link();
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(scans(&harness), [true, false, true]);
}

#[tokio::test(start_paused = true)]
async fn scanning_restarts_after_the_adapter_returns() {
    let harness = Harness::new();
    let lease_id = lease(&harness).await;
    settle().await;
    harness.core.on_adapter_state(AdapterStatus {
        state: AdapterState::PoweredOff,
        message: None,
    });
    settle().await;
    let list = harness.ok("/local/pod/discovery/list", json!({})).await;
    assert_eq!(list["adapter"]["state"], "poweredOff");
    assert_eq!(list["scanning"], false);
    harness.core.on_adapter_state(ready());
    harness
        .ok("/local/pod/discovery/renew", json!({ "leaseId": lease_id }))
        .await;
    tokio::time::sleep(Duration::from_secs(2)).await;
    assert_eq!(scans(&harness).last(), Some(&true));
    assert_eq!(
        scans(&harness).iter().filter(|scanning| **scanning).count(),
        2
    );
}

#[tokio::test(start_paused = true)]
async fn a_refused_scan_reports_the_adapter_and_retries() {
    let harness = Harness::new();
    harness.radio.refuse.store(true, Ordering::SeqCst);
    let lease_id = lease(&harness).await;
    settle().await;
    let list: Value = harness.ok("/local/pod/discovery/list", json!({})).await;
    assert_eq!(list["adapter"]["state"], "denied");
    assert_eq!(list["scanning"], false);
    harness.radio.refuse.store(false, Ordering::SeqCst);
    harness.core.on_adapter_state(ready());
    tokio::time::sleep(Duration::from_secs(6)).await;
    harness
        .ok("/local/pod/discovery/renew", json!({ "leaseId": lease_id }))
        .await;
    let list = harness.ok("/local/pod/discovery/list", json!({})).await;
    assert_eq!(list["scanning"], true);
}

#[tokio::test(start_paused = true)]
async fn bluetooth_settings_are_opened_by_the_platform() {
    let harness = Harness::new();
    let opened = harness.ok("/local/pod/bluetooth/settings", json!({})).await;
    assert_eq!(opened, json!({ "opened": true }));
}
