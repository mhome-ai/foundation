use app_facade_api::pod::{
    CommissionSession, CommissionState, CredentialStatus, CredentialStatusResponse, DeviceKind,
    DiscoverySnapshot, RequestErrorReason, ACTIVATION_ERROR_CODES, ACTIVATION_GRACE_MS,
    APP_CAPABILITY, APP_INFO_LABEL, APP_INFO_LOCKED, APP_INFO_LOCKED_FOR_MS, BLE_COMPANY_ID,
    BLE_SERVICE_UUID, CANDIDATE_TTL_MS, CODE_ATTEMPTS, CREDENTIAL_ISSUE_PATH,
    CREDENTIAL_REFUSAL_REASONS, CREDENTIAL_REVOKE_PATH, CREDENTIAL_STATUS_PATH,
    CREDENTIAL_STATUS_POLL_MS, CUSTOM_ENDPOINT_MAX_BYTES, DISCOVERY_EVENT_INTERVAL_MS,
    DISCOVERY_LEASE_MS, EVENT_TARGETS, LOCAL_TARGETS, NOT_ACTIVATED_DETAIL, POD_ENDPOINT_UUIDS,
    POD_MODE_REPROVISION, SECURITY_VERSION, SESSION_LEASE_MS, SESSION_LIFETIME_MS,
    SESSION_RENEW_INTERVAL_MS, SRP_USERNAME, STANDARD_ENDPOINT_UUIDS,
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
    assert_eq!(ble["securityVersion"], SECURITY_VERSION);
    assert_eq!(ble["appInfoLabel"], APP_INFO_LABEL);
    assert_eq!(ble["appCapability"], APP_CAPABILITY);
    assert_eq!(ble["appInfoLocked"], APP_INFO_LOCKED);
    assert_eq!(ble["appInfoLockedForMs"], APP_INFO_LOCKED_FOR_MS);
    assert_eq!(ble["codeAttempts"], CODE_ATTEMPTS);
    assert_eq!(manifest["discovery"]["leaseMs"], DISCOVERY_LEASE_MS);
    assert_eq!(manifest["discovery"]["candidateTtlMs"], CANDIDATE_TTL_MS);
    assert_eq!(
        manifest["discovery"]["eventIntervalMs"],
        DISCOVERY_EVENT_INTERVAL_MS
    );
    let session = &manifest["session"];
    assert_eq!(session["lifetimeMs"], SESSION_LIFETIME_MS);
    assert_eq!(session["leaseMs"], SESSION_LEASE_MS);
    assert_eq!(session["renewIntervalMs"], SESSION_RENEW_INTERVAL_MS);
    assert_eq!(session["activationGraceMs"], ACTIVATION_GRACE_MS);
    assert_eq!(session["credentialStatusPollMs"], CREDENTIAL_STATUS_POLL_MS);
    assert_eq!(manifest["cloud"]["issue"], CREDENTIAL_ISSUE_PATH);
    assert_eq!(manifest["cloud"]["status"], CREDENTIAL_STATUS_PATH);
    assert_eq!(manifest["cloud"]["revoke"], CREDENTIAL_REVOKE_PATH);
    assert_eq!(
        strings(&manifest["activationErrorCodes"]),
        ACTIVATION_ERROR_CODES
    );
    assert_eq!(manifest["notActivatedDetail"], NOT_ACTIVATED_DETAIL);
    assert_eq!(
        strings(&manifest["credentialRefusalReasons"]),
        CREDENTIAL_REFUSAL_REASONS
    );
    assert_eq!(manifest["reprovisionMode"], POD_MODE_REPROVISION);
    assert_eq!(
        strings(&manifest["requestErrorReasons"]),
        RequestErrorReason::ALL
            .iter()
            .map(|reason| reason.as_str())
            .collect::<Vec<_>>()
    );
    let contract = include_str!("../contract/pod-commissioning-v1.md");
    assert!(contract.contains(BLE_SERVICE_UUID));
    for target in LOCAL_TARGETS {
        assert!(contract.contains(target), "{target}");
    }
    for reason in RequestErrorReason::ALL {
        assert!(contract.contains(reason.as_str()), "{reason:?}");
    }
    for code in ACTIVATION_ERROR_CODES
        .iter()
        .chain(CREDENTIAL_REFUSAL_REASONS)
        .chain([&APP_INFO_LOCKED_FOR_MS, &POD_MODE_REPROVISION])
    {
        assert!(contract.contains(code), "{code}");
    }
    assert!(contract.contains(CREDENTIAL_STATUS_PATH));
}

#[test]
fn credential_status_matches_its_schema() {
    let schema: Value = serde_json::from_str(include_str!(
        "../schema/pod-credential-status.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (raw, status) in [
        (
            serde_json::json!({ "podId": "p", "status": "active", "activatedAt": 1791300000000i64 }),
            CredentialStatus::Active,
        ),
        (
            serde_json::json!({ "podId": "p", "status": "pending" }),
            CredentialStatus::Pending,
        ),
        (
            serde_json::json!({ "podId": "p", "status": "absent" }),
            CredentialStatus::Absent,
        ),
    ] {
        assert!(validator.is_valid(&raw), "{raw}");
        let parsed: CredentialStatusResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.status, status);
    }
    assert!(!validator.is_valid(&serde_json::json!({ "podId": "p", "status": "active" })));
    assert!(!validator.is_valid(&serde_json::json!({ "podId": "p", "status": "revoked" })));
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
        (
            include_str!("../fixtures/pod-commission.reprovision-completed.json"),
            CommissionState::Completed,
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

    let reprovisioned: Value = serde_json::from_str(include_str!(
        "../fixtures/pod-commission.reprovision-completed.json"
    ))
    .unwrap();
    let mut plain = reprovisioned.clone();
    plain.as_object_mut().unwrap().remove("mode");
    assert!(!commission.is_valid(&plain));
    assert!(serde_json::from_value::<CommissionSession>(plain)
        .unwrap()
        .validate()
        .is_err());
    let mut issued = reprovisioned;
    issued["podId"] = Value::from("pod-1");
    assert!(!commission.is_valid(&issued));
    assert!(serde_json::from_value::<CommissionSession>(issued)
        .unwrap()
        .validate()
        .is_err());
}
