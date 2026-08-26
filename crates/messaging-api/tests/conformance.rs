use messaging_api::{
    ConnectionListResponse, ProviderListRequest, ProviderListResponse, SetupResponse,
    SetupStartRequest, MANAGEMENT_TARGETS,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

const FIXTURES: &[(&str, &str)] = &[
    (
        "provider-list.request.json",
        include_str!("../fixtures/provider-list.request.json"),
    ),
    (
        "provider-list.response.json",
        include_str!("../fixtures/provider-list.response.json"),
    ),
    (
        "setup-start.request.json",
        include_str!("../fixtures/setup-start.request.json"),
    ),
    (
        "setup-status.response.json",
        include_str!("../fixtures/setup-status.response.json"),
    ),
    (
        "connection-list.response.json",
        include_str!("../fixtures/connection-list.response.json"),
    ),
];

fn body<T: DeserializeOwned>(raw: &str) -> T {
    let frame: Value = serde_json::from_str(raw).unwrap();
    serde_json::from_value(frame["body"].clone()).unwrap()
}

#[test]
fn fixtures_match_schema_and_typed_contracts() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/messaging-frame.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (name, raw) in FIXTURES {
        let frame: Value = serde_json::from_str(raw).unwrap();
        let errors = validator
            .iter_errors(&frame)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "fixture {name} failed: {errors:?}");
    }

    body::<ProviderListRequest>(FIXTURES[0].1);
    body::<ProviderListResponse>(FIXTURES[1].1);
    body::<SetupStartRequest>(FIXTURES[2].1);
    body::<SetupResponse>(FIXTURES[3].1);
    body::<ConnectionListResponse>(FIXTURES[4].1);
}

#[test]
fn target_manifest_matches_rust_inventory() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/targets.v1.json")).unwrap();
    let targets = manifest["requestTargets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(targets, MANAGEMENT_TARGETS);
}
