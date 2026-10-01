use core_api::host::permissions::*;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn automation_grants_are_distinct_per_target_app() {
    let keys: BTreeSet<PermissionKey> = [
        json!({"id":"automation", "targetBundleId":"com.apple.Music"}),
        json!({"id":"automation", "targetBundleId":"com.apple.iCal"}),
        json!({"id":"automation", "targetBundleId":"com.apple.Music"}),
    ]
    .into_iter()
    .map(|v| serde_json::from_value(v).unwrap())
    .collect();
    assert_eq!(keys.len(), 2);
    assert!(serde_json::from_value::<PermissionKey>(json!({"id":"automation"})).is_err());
    assert!(serde_json::from_value::<PermissionKey>(
        json!({"id":"microphone", "targetBundleId":"com.apple.Music"})
    )
    .is_err());
}

#[test]
fn local_actions_do_not_accept_caller_asserted_locality_or_settings_urls() {
    let request = json!({"hostId":"h", "componentId":"matter", "permission":{"id":"bluetooth"}});
    assert!(serde_json::from_value::<HostPermissionRequest>(request.clone()).is_ok());
    for (key, value) in [
        ("isLocal", json!(true)),
        ("scopeId", json!("s")),
        ("url", json!("https://example.com")),
    ] {
        let mut invalid = request.clone();
        invalid[key] = value;
        assert!(serde_json::from_value::<HostPermissionRequest>(invalid).is_err());
    }
}

#[test]
fn installed_requirements_are_os_specific_and_feature_scoped() {
    let requirements: PlatformPermissionRequirements = serde_json::from_value(json!({
        "macos": [{"permission":{"id":"microphone"}, "feature":"Audio recording", "reason":"Read audio input"}],
        "linux": []
    })).unwrap();
    assert!(requirements["linux"].is_empty());
    assert_eq!(
        requirements["macos"][0].permission,
        PermissionKey::Microphone {}
    );
    assert_eq!(requirements["macos"][0].feature, "Audio recording");
}

#[test]
fn permission_actions_require_the_selected_component() {
    let request: HostPermissionRequest = serde_json::from_value(json!({
        "hostId":"h", "componentId":"camera", "permission":{"id":"microphone"}
    }))
    .unwrap();
    assert_eq!(request.component_id, "camera");
    assert_eq!(
        serde_json::to_value(&request).unwrap()["componentId"],
        "camera"
    );
    assert!(serde_json::from_value::<HostPermissionRequest>(json!({
        "hostId":"h", "permission":{"id":"microphone"}
    }))
    .is_err());
    assert!(serde_json::from_value::<HostPermissionRequest>(json!({
        "hostId":"h", "componentId":42, "permission":{"id":"microphone"}
    }))
    .is_err());
}

#[test]
fn rows_preserve_each_components_authorization_without_grouping() {
    let row = |id: &str, state| HostPermission {
        component_id: id.into(),
        component_name: id.into(),
        permission: PermissionKey::Bluetooth {},
        feature: "Discovery".into(),
        reason: "Discover devices".into(),
        observation: Some(PermissionObservation {
            permission: PermissionKey::Bluetooth {},
            process_id: 101,
            state,
            evidence: PermissionEvidence::System,
            observed_at_ms: Some(123),
            error: None,
        }),
        error: None,
        supported_actions: vec![PermissionAction::Request],
    };
    let granted = row("matter", PermissionState::Granted);
    let denied = row("other", PermissionState::Denied);
    assert!(granted.is_satisfied());
    assert!(!denied.is_satisfied());
    let mut stopped = row("stopped", PermissionState::Granted);
    stopped.observation = None;
    stopped.supported_actions.clear();
    assert!(!stopped.is_satisfied());
    let snapshot = json!({"hostId":"h", "hostVersion":"1", "platform":"macos", "observedAtMs":123,
        "permissions":[granted, denied, stopped], "declarationErrors":[]});
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/host-permissions.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&snapshot));
    for (key, value) in [
        ("subject", json!(null)),
        ("uses", json!([])),
        ("requestComponentId", json!("matter")),
    ] {
        let mut invalid = snapshot.clone();
        invalid["permissions"][0][key] = value;
        assert!(!validator.is_valid(&invalid));
        assert!(serde_json::from_value::<HostPermissions>(invalid).is_err());
    }
}

#[test]
fn only_valid_authorization_observations_satisfy_a_requirement() {
    let mut observed = PermissionObservation {
        permission: PermissionKey::LocalNetwork {},
        process_id: 1,
        state: PermissionState::NotRequired,
        evidence: PermissionEvidence::Platform,
        observed_at_ms: Some(123),
        error: None,
    };
    assert!(observed.is_satisfied());
    for state in [
        PermissionState::Unknown,
        PermissionState::NotDetermined,
        PermissionState::Denied,
        PermissionState::Restricted,
        PermissionState::Unsupported,
    ] {
        observed.state = state;
        assert!(!observed.is_satisfied());
    }
    observed.state = PermissionState::Granted;
    observed.evidence = PermissionEvidence::Probe;
    observed.observed_at_ms = None;
    assert!(!observed.is_satisfied());
    observed.observed_at_ms = Some(123);
    assert!(observed.is_satisfied());
    observed.error = Some("Query failed".into());
    assert!(!observed.is_satisfied());
}

#[test]
fn permission_reports_only_describe_authorization() {
    let observed = json!({
        "permission": {"id":"bluetooth"},
        "processId": 101,
        "state":"notRequired",
        "evidence":"platform",
        "observedAtMs":123,
        "error":null
    });
    let decoded: PermissionObservation = serde_json::from_value(observed.clone()).unwrap();
    assert_eq!(decoded.state, PermissionState::NotRequired);
    assert_eq!(serde_json::to_value(decoded).unwrap(), observed);
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/host-permissions.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({"processId":101, "permissions":[observed.clone()]})));
    for resource_state in [
        serde_json::Value::Null,
        json!({"state":"available", "observedAtMs":123, "detail":"Transport initialized"}),
    ] {
        let mut invalid = observed.clone();
        invalid["access"] = resource_state;
        assert!(serde_json::from_value::<PermissionObservation>(invalid.clone()).is_err());
        assert!(!validator.is_valid(&json!({"processId":101, "permissions":[invalid]})));
    }
}

#[test]
fn npm_wire_schema_rejects_legacy_actions_and_undated_probes() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/host-permissions.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(!validator.is_valid(&json!({"hostId":"h", "permission":{"id":"bluetooth"}})));
    assert!(validator
        .is_valid(&json!({"hostId":"h", "componentId":"matter", "permission":{"id":"bluetooth"}})));
    let mut report = json!({"processId":10, "permissions":[{
        "permission":{"id":"localNetwork"}, "processId":11, "state":"granted", "evidence":"probe", "observedAtMs":null, "error":null
    }]});
    assert!(!validator.is_valid(&report));
    report["permissions"][0]["observedAtMs"] = json!(123);
    assert!(validator.is_valid(&report));
    let decoded: ServicePermissions = serde_json::from_value(report).unwrap();
    assert_eq!(decoded.process_id, 10);
    assert_eq!(decoded.permissions[0].process_id, 11);
}
