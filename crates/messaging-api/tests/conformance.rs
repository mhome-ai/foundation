use messaging_api::{
    AccountBindingListResponse, AccountBindingRequest, ConnectionListResponse, ConnectionRequest,
    ConnectionStatusResponse, ConnectionTestRequest, ConnectionTestResponse,
    ConnectionUpdateRequest, ConnectionUpdateResponse, MutationResponse, NormalizedInbound,
    ProviderListRequest, ProviderListResponse, ProviderPlacementRequest, SetupOptionsResponse,
    SetupResponse, SetupStartRequest, SetupStatusRequest, MANAGEMENT_TARGETS,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

const VALID_FIXTURES: &[(&str, &str)] = &[
    (
        "provider-list.request.json",
        include_str!("../fixtures/provider-list.request.json"),
    ),
    (
        "provider-list.response.json",
        include_str!("../fixtures/provider-list.response.json"),
    ),
    (
        "connection-list.request.json",
        include_str!("../fixtures/connection-list.request.json"),
    ),
    (
        "connection-list.response.json",
        include_str!("../fixtures/connection-list.response.json"),
    ),
    (
        "connection-update.request.json",
        include_str!("../fixtures/connection-update.request.json"),
    ),
    (
        "connection-update.response.json",
        include_str!("../fixtures/connection-update.response.json"),
    ),
    (
        "connection-delete.request.json",
        include_str!("../fixtures/connection-delete.request.json"),
    ),
    (
        "connection-delete.response.json",
        include_str!("../fixtures/connection-delete.response.json"),
    ),
    (
        "connection-test.request.json",
        include_str!("../fixtures/connection-test.request.json"),
    ),
    (
        "connection-test.response.json",
        include_str!("../fixtures/connection-test.response.json"),
    ),
    (
        "connection-status.request.json",
        include_str!("../fixtures/connection-status.request.json"),
    ),
    (
        "connection-status.response.json",
        include_str!("../fixtures/connection-status.response.json"),
    ),
    (
        "account-bind.request.json",
        include_str!("../fixtures/account-bind.request.json"),
    ),
    (
        "account-bind.response.json",
        include_str!("../fixtures/account-bind.response.json"),
    ),
    (
        "account-unbind.request.json",
        include_str!("../fixtures/account-unbind.request.json"),
    ),
    (
        "account-unbind.response.json",
        include_str!("../fixtures/account-unbind.response.json"),
    ),
    (
        "account-binding-list.request.json",
        include_str!("../fixtures/account-binding-list.request.json"),
    ),
    (
        "account-binding-list.response.json",
        include_str!("../fixtures/account-binding-list.response.json"),
    ),
    (
        "setup-options.request.json",
        include_str!("../fixtures/setup-options.request.json"),
    ),
    (
        "setup-options.response.json",
        include_str!("../fixtures/setup-options.response.json"),
    ),
    (
        "setup-start.request.json",
        include_str!("../fixtures/setup-start.request.json"),
    ),
    (
        "setup-start.response.json",
        include_str!("../fixtures/setup-start.response.json"),
    ),
    (
        "setup-status.request.json",
        include_str!("../fixtures/setup-status.request.json"),
    ),
    (
        "setup-status.response.json",
        include_str!("../fixtures/setup-status.response.json"),
    ),
];

const INVALID_FIXTURES: &[(&str, &str)] = &[
    (
        "invalid/missing-required-extension.json",
        include_str!("../fixtures/invalid/missing-required-extension.json"),
    ),
    (
        "invalid/unknown-request-field.json",
        include_str!("../fixtures/invalid/unknown-request-field.json"),
    ),
    (
        "invalid/mismatched-direction.json",
        include_str!("../fixtures/invalid/mismatched-direction.json"),
    ),
];

fn body<T: DeserializeOwned>(name: &str) -> T {
    let raw = VALID_FIXTURES
        .iter()
        .find_map(|(candidate, raw)| (*candidate == name).then_some(*raw))
        .unwrap_or_else(|| panic!("missing fixture: {name}"));
    let frame: Value = serde_json::from_str(raw).unwrap();
    serde_json::from_value(frame["body"].clone()).unwrap()
}

#[test]
fn normalized_inbound_fixture_matches_schema_and_semantic_validation() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/normalized-inbound.v1.schema.json")).unwrap();
    let fixture: Value = serde_json::from_str(include_str!(
        "../fixtures/normalized-inbound.shared-text.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let errors = validator
        .iter_errors(&fixture)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "normalized inbound fixture failed: {errors:?}"
    );
    let inbound: NormalizedInbound = serde_json::from_value(fixture).unwrap();
    assert!(inbound.validate().is_ok());
}

#[test]
fn normalized_inbound_corpus_seals_schema_and_semantics() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/normalized-inbound.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/normalized-inbound.conformance.json"
    ))
    .unwrap();

    for case in corpus["valid"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let value = case["value"].clone();
        assert!(
            validator.is_valid(&value),
            "valid case failed schema: {name}"
        );
        let inbound: NormalizedInbound = serde_json::from_value(value).unwrap();
        assert!(
            inbound.validate().is_ok(),
            "valid case failed semantics: {name}"
        );
    }

    for case in corpus["invalid"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let value = case["value"].clone();
        let rejected = !validator.is_valid(&value)
            || serde_json::from_value::<NormalizedInbound>(value)
                .map_or(true, |inbound| inbound.validate().is_err());
        assert!(rejected, "invalid case was accepted: {name}");
    }
}

#[test]
fn every_management_target_has_typed_request_and_response_fixtures() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/messaging-frame.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (name, raw) in VALID_FIXTURES {
        let frame: Value = serde_json::from_str(raw).unwrap();
        let errors = validator
            .iter_errors(&frame)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(errors.is_empty(), "fixture {name} failed: {errors:?}");
    }

    for target in MANAGEMENT_TARGETS {
        assert!(
            VALID_FIXTURES.iter().any(|(_, raw)| {
                let frame: Value = serde_json::from_str(raw).unwrap();
                frame["target"] == *target && frame["direction"] == "request"
            }),
            "missing request fixture for {target}"
        );
        assert!(
            VALID_FIXTURES.iter().any(|(_, raw)| {
                let frame: Value = serde_json::from_str(raw).unwrap();
                frame["target"] == *target && frame["direction"] == "response"
            }),
            "missing response fixture for {target}"
        );
    }

    body::<ProviderListRequest>("provider-list.request.json");
    body::<ProviderListResponse>("provider-list.response.json");
    body::<ProviderPlacementRequest>("connection-list.request.json");
    body::<ConnectionListResponse>("connection-list.response.json");
    body::<ConnectionUpdateRequest>("connection-update.request.json");
    body::<ConnectionUpdateResponse>("connection-update.response.json");
    body::<ConnectionRequest>("connection-delete.request.json");
    body::<MutationResponse>("connection-delete.response.json");
    body::<ConnectionTestRequest>("connection-test.request.json");
    body::<ConnectionTestResponse>("connection-test.response.json");
    body::<ProviderPlacementRequest>("connection-status.request.json");
    body::<ConnectionStatusResponse>("connection-status.response.json");
    body::<AccountBindingRequest>("account-bind.request.json");
    body::<MutationResponse>("account-bind.response.json");
    body::<AccountBindingRequest>("account-unbind.request.json");
    body::<MutationResponse>("account-unbind.response.json");
    body::<AccountBindingRequest>("account-binding-list.request.json");
    body::<AccountBindingListResponse>("account-binding-list.response.json");
    body::<ProviderPlacementRequest>("setup-options.request.json");
    body::<SetupOptionsResponse>("setup-options.response.json");
    body::<SetupStartRequest>("setup-start.request.json");
    body::<SetupResponse>("setup-start.response.json");
    body::<SetupStatusRequest>("setup-status.request.json");
    body::<SetupResponse>("setup-status.response.json");
}

#[test]
fn invalid_frames_are_rejected() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/messaging-frame.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (name, raw) in INVALID_FIXTURES {
        let frame: Value = serde_json::from_str(raw).unwrap();
        assert!(
            !validator.is_valid(&frame),
            "fixture {name} unexpectedly passed"
        );
    }

    let missing: Value = serde_json::from_str(INVALID_FIXTURES[0].1).unwrap();
    assert!(
        serde_json::from_value::<ConnectionTestRequest>(missing["body"].clone()).is_err(),
        "Rust DTO unexpectedly accepted a missing required extension object"
    );
}

#[test]
fn target_manifest_matches_rust_inventory() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/targets.v1.json")).unwrap();
    for field in ["requestTargets", "responseTargets"] {
        let targets = manifest[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(targets, MANAGEMENT_TARGETS, "manifest field {field}");
    }
}
