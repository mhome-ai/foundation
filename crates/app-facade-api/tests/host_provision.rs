use app_facade_api::host_provision::{
    HostFinishResponse, ProvisionSession, ProvisionState, APP_CAPABILITY, APP_INFO_LABEL,
    APP_INFO_LOCKED, CLAIM_TOKEN_LEN, CODE_LOCKOUT_MAX_MS, CODE_LOCKOUT_MS, EVENT_TARGETS,
    HOST_ENDPOINT_UUIDS, LOCAL_TARGETS,
};
use app_facade_api::pod::{DeviceKind, BLE_KIND_HOST};
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
        serde_json::from_str(include_str!("../manifest/host-provision-targets.v1.json")).unwrap();
    assert_eq!(strings(&manifest["localTargets"]), LOCAL_TARGETS);
    assert_eq!(strings(&manifest["eventTargets"]), EVENT_TARGETS);
    assert_eq!(manifest["localPayload"], "domainInput");
    assert_eq!(manifest["localForwardable"], false);
    assert_eq!(manifest["ble"]["kind"], BLE_KIND_HOST);
    for (name, uuid) in HOST_ENDPOINT_UUIDS {
        assert_eq!(
            manifest["ble"]["customEndpointUuids"][name], *uuid,
            "{name}"
        );
    }
    assert_eq!(manifest["ble"]["appInfoLabel"], APP_INFO_LABEL);
    assert_eq!(manifest["ble"]["appCapability"], APP_CAPABILITY);
    assert_eq!(manifest["ble"]["appInfoLocked"], APP_INFO_LOCKED);
    assert_eq!(manifest["codeLockout"]["initialMs"], CODE_LOCKOUT_MS);
    assert_eq!(manifest["codeLockout"]["maxMs"], CODE_LOCKOUT_MAX_MS);
    assert_eq!(manifest["claimTokenLength"], CLAIM_TOKEN_LEN);
    let contract = include_str!("../contract/host-provisioning-v1.md");
    for target in LOCAL_TARGETS {
        assert!(contract.contains(target), "{target}");
    }
    assert!(contract.contains("claimToken"));
    let facade: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    assert_eq!(
        facade["domains"]["hostProvision"]["targets"],
        "manifest/host-provision-targets.v1.json"
    );
}

fn validator() -> jsonschema::Validator {
    let discovery: Value =
        serde_json::from_str(include_str!("../schema/pod-discovery.v1.schema.json")).unwrap();
    let provision: Value =
        serde_json::from_str(include_str!("../schema/host-provision.v1.schema.json")).unwrap();
    jsonschema::options()
        .with_resource(
            "https://schemas.mhome.ai/app-facade/pod-discovery.v1.schema.json",
            jsonschema::Resource::from_contents(discovery).unwrap(),
        )
        .build(&provision)
        .unwrap()
}

#[test]
fn fixture_matches_type_and_schema() {
    let schema = validator();
    let raw: Value =
        serde_json::from_str(include_str!("../fixtures/host-provision.completed.json")).unwrap();
    assert!(schema.is_valid(&raw), "{raw}");
    let session: ProvisionSession = serde_json::from_value(raw.clone()).unwrap();
    assert_eq!(session.state, ProvisionState::Completed);
    assert_eq!(session.candidate.kind, DeviceKind::Host);
    session.validate().unwrap();

    assert!(session
        .claim_token
        .as_deref()
        .is_some_and(|token| token.len() == CLAIM_TOKEN_LEN));

    let mut bad_token = raw.clone();
    bad_token["claimToken"] = Value::from("short");
    assert!(!schema.is_valid(&bad_token));
    let session: ProvisionSession = serde_json::from_value(bad_token).unwrap();
    assert!(session.validate().is_err());

    let mut early_token = raw.clone();
    early_token["state"] = Value::from("joining_wifi");
    assert!(!schema.is_valid(&early_token));
    let session: ProvisionSession = serde_json::from_value(early_token).unwrap();
    assert!(session.validate().is_err());

    let mut no_addresses = raw.clone();
    no_addresses.as_object_mut().unwrap().remove("addresses");
    assert!(schema.is_valid(&no_addresses));

    let finish: HostFinishResponse =
        serde_json::from_value(serde_json::json!({ "ok": true, "claimToken": raw["claimToken"] }))
            .unwrap();
    assert_eq!(finish.claim_token.as_deref(), raw["claimToken"].as_str());

    let mut no_host = raw.clone();
    no_host.as_object_mut().unwrap().remove("host");
    assert!(!schema.is_valid(&no_host));
    let session: ProvisionSession = serde_json::from_value(no_host).unwrap();
    assert!(session.validate().is_err());

    for (state, code, valid) in [
        ("failed", Some("disconnected"), true),
        ("failed", None, false),
        ("awaiting_code", Some("code_rejected"), true),
        ("awaiting_wifi", Some("wifi_not_found"), true),
        ("awaiting_wifi", Some("code_rejected"), false),
        ("failed", Some("issue_failed"), false),
    ] {
        let mut value = raw.clone();
        value["state"] = Value::from(state);
        value.as_object_mut().unwrap().remove("host");
        value.as_object_mut().unwrap().remove("claimToken");
        if let Some(code) = code {
            value["error"] = serde_json::json!({ "code": code, "message": "m" });
        }
        assert_eq!(schema.is_valid(&value), valid, "{state} {code:?}");
        if let Ok(session) = serde_json::from_value::<ProvisionSession>(value) {
            assert_eq!(session.validate().is_ok(), valid, "{state} {code:?}");
        } else {
            assert!(!valid);
        }
    }
}
