use app_facade_api::pod::{
    CommissionSession, CommissionState, DeviceKind, DiscoverySnapshot, BLE_COMPANY_ID,
    BLE_SERVICE_UUID, CUSTOM_ENDPOINT_MAX_BYTES, EVENT_TARGETS, LOCAL_TARGETS, POD_ENDPOINT_UUIDS,
    SRP_USERNAME, STANDARD_ENDPOINT_UUIDS,
};
use serde_json::Value;

fn strings(value: &Value) -> Vec<&str> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item.as_str().unwrap())
        .collect()
}

#[test]
fn manifest_matches_the_rust_contract() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/pod-targets.v1.json")).unwrap();
    assert_eq!(strings(&manifest["localTargets"]), LOCAL_TARGETS);
    assert_eq!(strings(&manifest["eventTargets"]), EVENT_TARGETS);
    assert_eq!(manifest["localPayload"], "domainInput");
    assert_eq!(manifest["localForwardable"], false);
    let ble = &manifest["ble"];
    assert_eq!(ble["serviceUuid"], BLE_SERVICE_UUID);
    assert_eq!(ble["companyId"], BLE_COMPANY_ID);
    assert_eq!(ble["srpUsername"], SRP_USERNAME);
    assert_eq!(ble["customEndpointMaxBytes"], CUSTOM_ENDPOINT_MAX_BYTES);
    for (name, uuid) in STANDARD_ENDPOINT_UUIDS {
        assert_eq!(ble["standardEndpointUuids"][name], *uuid, "{name}");
    }
    for (name, uuid) in POD_ENDPOINT_UUIDS {
        assert_eq!(ble["customEndpointUuids"][name], *uuid, "{name}");
    }
    assert!(include_str!("../contract/pod-commissioning-v1.md").contains(BLE_SERVICE_UUID));
}

#[test]
fn fixtures_match_types_and_schemas() {
    let discovery_schema: Value =
        serde_json::from_str(include_str!("../schema/pod-discovery.v1.schema.json")).unwrap();
    let commission_schema: Value =
        serde_json::from_str(include_str!("../schema/pod-commission.v1.schema.json")).unwrap();
    let discovery = jsonschema::validator_for(&discovery_schema).unwrap();
    let commission = jsonschema::options()
        .with_resource(
            "https://schemas.mhome.ai/app-facade/pod-discovery.v1.schema.json",
            jsonschema::Resource::from_contents(discovery_schema.clone()).unwrap(),
        )
        .build(&commission_schema)
        .unwrap();

    let raw: Value =
        serde_json::from_str(include_str!("../fixtures/pod-discovery.snapshot.json")).unwrap();
    assert!(discovery.is_valid(&raw));
    let snapshot: DiscoverySnapshot = serde_json::from_value(raw).unwrap();
    assert_eq!(snapshot.candidates[0].short_id, "1A2B3C4D");
    assert_eq!(snapshot.candidates[0].kind, DeviceKind::Pod);

    for (fixture, state) in [
        (
            include_str!("../fixtures/pod-commission.awaiting-authorization.json"),
            CommissionState::AwaitingAuthorization,
        ),
        (
            include_str!("../fixtures/pod-commission.failed.json"),
            CommissionState::Failed,
        ),
    ] {
        let raw: Value = serde_json::from_str(fixture).unwrap();
        assert!(commission.is_valid(&raw), "{raw}");
        let session: CommissionSession = serde_json::from_value(raw).unwrap();
        assert_eq!(session.state, state);
        session.validate().unwrap();
    }

    let mut missing_error: Value =
        serde_json::from_str(include_str!("../fixtures/pod-commission.failed.json")).unwrap();
    missing_error.as_object_mut().unwrap().remove("error");
    assert!(!commission.is_valid(&missing_error));
    let session: CommissionSession = serde_json::from_value(missing_error).unwrap();
    assert!(session.validate().is_err());

    let failed: Value =
        serde_json::from_str(include_str!("../fixtures/pod-commission.failed.json")).unwrap();
    for (state, code, valid) in [
        ("awaiting_code", "code_rejected", true),
        ("awaiting_wifi", "wifi_auth_failed", true),
        ("awaiting_code", "wifi_failed", false),
        ("reading_info", "code_rejected", false),
    ] {
        let mut retry = failed.clone();
        retry["state"] = Value::from(state);
        retry["error"]["code"] = Value::from(code);
        assert_eq!(commission.is_valid(&retry), valid, "{state} {code}");
        let session: CommissionSession = serde_json::from_value(retry).unwrap();
        assert_eq!(session.validate().is_ok(), valid, "{state} {code}");
    }
}
