use app_facade_api::pod::{
    CommissionSession, CommissionState, DiscoverySnapshot, BLE_COMPANY_ID, BLE_SERVICE_UUID,
    CUSTOM_ENDPOINT_MAX_BYTES, EVENT_TARGETS, LOCAL_TARGETS, SRP_USERNAME,
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
}
