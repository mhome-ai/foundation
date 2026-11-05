use core_api::extension_sources::*;

#[test]
fn shared_source_conformance() {
    let fixtures: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/extension-sources.conformance.json"
    ))
    .unwrap();
    for case in fixtures["cases"].as_array().unwrap() {
        let value = case["value"].clone();
        let valid = match case["kind"].as_str().unwrap() {
            "source" => serde_json::from_value::<IntegrationSource>(value)
                .ok()
                .is_some_and(|v| v.validate().is_ok()),
            "snapshot" => serde_json::from_value::<SpaceSources>(value)
                .ok()
                .is_some_and(|v| v.validate().is_ok()),
            "material" => serde_json::from_value::<SourceReadResponse>(value)
                .ok()
                .is_some_and(|v| v.validate().is_ok()),
            other => panic!("Unknown fixture kind {other}"),
        };
        assert_eq!(valid, case["valid"].as_bool().unwrap(), "{}", case["name"]);
    }
}

#[test]
fn source_identity_distinguishes_local_snapshot_from_registry_version() {
    let published: IntegrationSource = serde_json::from_value(serde_json::json!({
        "integrationId":"garden.device", "version":7, "type":"standard"
    }))
    .unwrap();
    published.validate().unwrap();
    let mut preview = published.clone();
    preview.snapshot = Some("a".repeat(64).parse().unwrap());
    assert_ne!(published, preview);
    preview.validate().unwrap();
    let sources = SpaceSources {
        revision: 1,
        integrations: vec![published.clone(), preview.clone()],
    };
    assert!(sources.validate().is_err());
    assert!("A".repeat(64).parse::<SnapshotId>().is_err());
    preview.snapshot = None;
    preview.version = 0;
    assert!(preview.validate().is_err());
    let mut invalid = published.clone();
    invalid.integration_id = "../garden.device".into();
    assert!(invalid.validate().is_err());
    assert!(
        serde_json::from_value::<IntegrationSource>(serde_json::json!({
            "integrationId":"garden.device", "version":1.5, "type":"standard"
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<IntegrationSource>(serde_json::json!({
            "integrationId":"garden.device", "version":7, "type":"standard", "tenantId":"untrusted"
        }))
        .is_err()
    );
}

#[test]
fn material_inventory_is_bounded_and_retains_relative_schema_paths() {
    use std::collections::BTreeMap;
    let mut material = SourceReadResponse {
        source: IntegrationSource {
            integration_id: "garden.device".into(),
            version: 1,
            package_type: PackageType::Standard,
            snapshot: Some("a".repeat(64).parse().unwrap()),
        },
        schemas: BTreeMap::from([("capabilities/read.yml".into(), "capability: {}".into())]),
        i18n: BTreeMap::new(),
        script_digests: BTreeMap::from([("read".into(), "b".repeat(64))]),
    };
    material.validate().unwrap();
    for invalid in [
        "../read.yml",
        "/read.yml",
        "./read.yml",
        "a//read.yml",
        "a\\read.yml",
        "C:read.yml",
        "read.js",
    ] {
        material.schemas = BTreeMap::from([(invalid.into(), "x".into())]);
        assert!(material.validate().is_err(), "{invalid}");
    }
    material.schemas = BTreeMap::from([("read.yml".into(), "x".into())]);
    material
        .script_digests
        .insert("read".into(), "not-a-hash".into());
    assert!(material.validate().is_err());
    material.script_digests.clear();
    material
        .schemas
        .insert("read.yml".into(), "\n".repeat(MAX_SOURCE_BYTES / 2));
    assert!(
        material.validate().is_err(),
        "JSON escaping counts toward the transport bound"
    );
}
