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
    let request = json!({"hostId":"h", "permission":{"id":"bluetooth"}});
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
