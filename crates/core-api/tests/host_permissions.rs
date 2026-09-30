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
fn a_shared_subject_preserves_each_executors_observation() {
    let observation = |pid, state| PermissionObservation {
        permission: PermissionKey::Bluetooth {},
        process_id: pid,
        state,
        evidence: PermissionEvidence::System,
        observed_at_ms: Some(123),
        access: None,
        error: None,
    };
    let usage = |id: &str, observation| PermissionUse {
        component_id: id.into(),
        component_name: id.into(),
        feature: "Discovery".into(),
        reason: "Discover nearby devices".into(),
        observation,
        error: None,
    };
    let row = HostPermission {
        permission: PermissionKey::Bluetooth {},
        subject: Some(PermissionSubject {
            id: "ai.mhome.meowlink.hostd".into(),
            name: "MeowLink Host".into(),
        }),
        uses: vec![
            usage("matter", Some(observation(101, PermissionState::Granted))),
            usage(
                "second-service",
                Some(observation(102, PermissionState::Denied)),
            ),
            usage("stopped-service", None),
        ],
        request_component_id: Some("matter".into()),
        supported_actions: vec![PermissionAction::Request, PermissionAction::OpenSettings],
    };
    let wire = serde_json::to_value(&row).unwrap();
    assert!(
        wire.get("state").is_none(),
        "A group must not overwrite conflicting observations with one grant"
    );
    let snapshot = json!({"hostId":"h", "hostVersion":"1.0.0", "platform":"macos", "observedAtMs":123, "permissions":[wire.clone()], "declarationErrors":[]});
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/host-permissions.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&snapshot));
    let mut unknown_shared = snapshot.clone();
    unknown_shared["permissions"][0]["subject"] = serde_json::Value::Null;
    assert!(
        !validator.is_valid(&unknown_shared),
        "Unconfirmed subjects must stay separate"
    );
    let mut missing_executor = snapshot.clone();
    missing_executor["permissions"][0]["requestComponentId"] = serde_json::Value::Null;
    assert!(!validator.is_valid(&missing_executor));
    let decoded: HostPermission = serde_json::from_value(wire).unwrap();
    assert_eq!(
        decoded.uses[0].observation.as_ref().unwrap().process_id,
        101
    );
    assert_eq!(
        decoded.uses[1].observation.as_ref().unwrap().state,
        PermissionState::Denied
    );
    assert!(decoded.uses[2].observation.is_none());
}

#[test]
fn linux_access_failure_is_not_fabricated_user_denial() {
    let observed: PermissionObservation = serde_json::from_value(json!({
        "permission": {"id":"bluetooth"},
        "processId": 101,
        "state":"notRequired",
        "evidence":"platform",
        "observedAtMs":123,
        "access": {"state":"unavailable", "observedAtMs":123, "detail":"BlueZ rejected the scan request"},
        "error":null
    })).unwrap();
    assert_eq!(observed.state, PermissionState::NotRequired);
    assert_eq!(
        observed.access.unwrap().state,
        PermissionAccessState::Unavailable
    );
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
        "permission":{"id":"localNetwork"}, "processId":11, "state":"granted", "evidence":"probe", "observedAtMs":null, "access":null, "error":null
    }]});
    assert!(!validator.is_valid(&report));
    report["permissions"][0]["observedAtMs"] = json!(123);
    assert!(validator.is_valid(&report));
    let decoded: ServicePermissions = serde_json::from_value(report).unwrap();
    assert_eq!(decoded.process_id, 10);
    assert_eq!(decoded.permissions[0].process_id, 11);
}
