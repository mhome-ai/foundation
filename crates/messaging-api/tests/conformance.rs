use messaging_api::{
    AccountGrantListResponse, AccountGrantRequest, ActorLinkChallengeResponse, ActorLinkClaimEvent,
    ActorLinkClaimRequest, ActorLinkClaimResponse, ActorLinkCodeCreateRequest,
    ActorLinkDeleteRequest, ActorLinkListRequest, ActorLinkListResponse, ChallengeCodeResponse,
    MutationResponse, NormalizedInbound, ProviderAccountListResponse, ProviderAccountRequest,
    ProviderAccountStatusResponse, ProviderAccountTestRequest, ProviderAccountTestResponse,
    ProviderAccountUpdateRequest, ProviderAccountUpdateResponse, ProviderListRequest,
    ProviderListResponse, ProviderPlacementRequest, RouteListRequest, RouteListResponse,
    RouteRequest, RouteUpdateRequest, RouteUpdateResponse, SetupOptionsResponse, SetupResponse,
    SetupStartRequest, SetupStatusRequest, SurfaceBindCodeCreateRequest, SurfaceListRequest,
    SurfaceListResponse, SurfaceRequest, MANAGEMENT_TARGETS,
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
        "provider-account-list.request.json",
        include_str!("../fixtures/provider-account-list.request.json"),
    ),
    (
        "provider-account-list.response.json",
        include_str!("../fixtures/provider-account-list.response.json"),
    ),
    (
        "provider-account-update.request.json",
        include_str!("../fixtures/provider-account-update.request.json"),
    ),
    (
        "provider-account-update.response.json",
        include_str!("../fixtures/provider-account-update.response.json"),
    ),
    (
        "provider-account-delete.request.json",
        include_str!("../fixtures/provider-account-delete.request.json"),
    ),
    (
        "provider-account-delete.response.json",
        include_str!("../fixtures/provider-account-delete.response.json"),
    ),
    (
        "provider-account-test.request.json",
        include_str!("../fixtures/provider-account-test.request.json"),
    ),
    (
        "provider-account-test.response.json",
        include_str!("../fixtures/provider-account-test.response.json"),
    ),
    (
        "provider-account-status.request.json",
        include_str!("../fixtures/provider-account-status.request.json"),
    ),
    (
        "provider-account-status.response.json",
        include_str!("../fixtures/provider-account-status.response.json"),
    ),
    (
        "account-grant-create.request.json",
        include_str!("../fixtures/account-grant-create.request.json"),
    ),
    (
        "account-grant-create.response.json",
        include_str!("../fixtures/account-grant-create.response.json"),
    ),
    (
        "account-grant-delete.request.json",
        include_str!("../fixtures/account-grant-delete.request.json"),
    ),
    (
        "account-grant-delete.response.json",
        include_str!("../fixtures/account-grant-delete.response.json"),
    ),
    (
        "account-grant-list.request.json",
        include_str!("../fixtures/account-grant-list.request.json"),
    ),
    (
        "account-grant-list.response.json",
        include_str!("../fixtures/account-grant-list.response.json"),
    ),
    (
        "route-list.request.json",
        include_str!("../fixtures/route-list.request.json"),
    ),
    (
        "route-list.response.json",
        include_str!("../fixtures/route-list.response.json"),
    ),
    (
        "route-update.request.json",
        include_str!("../fixtures/route-update.request.json"),
    ),
    (
        "route-update.response.json",
        include_str!("../fixtures/route-update.response.json"),
    ),
    (
        "route-delete.request.json",
        include_str!("../fixtures/route-delete.request.json"),
    ),
    (
        "route-delete.response.json",
        include_str!("../fixtures/route-delete.response.json"),
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
    (
        "surface-list.request.json",
        include_str!("../fixtures/surface-list.request.json"),
    ),
    (
        "surface-list.response.json",
        include_str!("../fixtures/surface-list.response.json"),
    ),
    (
        "surface-dismiss.request.json",
        include_str!("../fixtures/surface-dismiss.request.json"),
    ),
    (
        "surface-dismiss.response.json",
        include_str!("../fixtures/surface-dismiss.response.json"),
    ),
    (
        "surface-bind-code-create.request.json",
        include_str!("../fixtures/surface-bind-code-create.request.json"),
    ),
    (
        "surface-bind-code-create.response.json",
        include_str!("../fixtures/surface-bind-code-create.response.json"),
    ),
    (
        "actor-link-code-create.request.json",
        include_str!("../fixtures/actor-link-code-create.request.json"),
    ),
    (
        "actor-link-code-create.response.json",
        include_str!("../fixtures/actor-link-code-create.response.json"),
    ),
    (
        "actor-link-claim-status.request.json",
        include_str!("../fixtures/actor-link-claim-status.request.json"),
    ),
    (
        "actor-link-claim-status.response.json",
        include_str!("../fixtures/actor-link-claim-status.response.json"),
    ),
    (
        "actor-link-claim-confirm.request.json",
        include_str!("../fixtures/actor-link-claim-confirm.request.json"),
    ),
    (
        "actor-link-claim-confirm.response.json",
        include_str!("../fixtures/actor-link-claim-confirm.response.json"),
    ),
    (
        "actor-link-claim-event.response.json",
        include_str!("../fixtures/actor-link-claim-event.response.json"),
    ),
    (
        "actor-link-list.request.json",
        include_str!("../fixtures/actor-link-list.request.json"),
    ),
    (
        "actor-link-list.response.json",
        include_str!("../fixtures/actor-link-list.response.json"),
    ),
    (
        "actor-link-delete.request.json",
        include_str!("../fixtures/actor-link-delete.request.json"),
    ),
    (
        "actor-link-delete.response.json",
        include_str!("../fixtures/actor-link-delete.response.json"),
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
        serde_json::from_str(include_str!("../schema/normalized-inbound.v4.schema.json")).unwrap();
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
        serde_json::from_str(include_str!("../schema/normalized-inbound.v4.schema.json")).unwrap();
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
        serde_json::from_str(include_str!("../schema/messaging-frame.v3.schema.json")).unwrap();
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
    body::<ProviderPlacementRequest>("provider-account-list.request.json");
    body::<ProviderAccountListResponse>("provider-account-list.response.json");
    body::<ProviderAccountUpdateRequest>("provider-account-update.request.json");
    body::<ProviderAccountUpdateResponse>("provider-account-update.response.json");
    body::<ProviderAccountRequest>("provider-account-delete.request.json");
    body::<MutationResponse>("provider-account-delete.response.json");
    body::<ProviderAccountTestRequest>("provider-account-test.request.json");
    body::<ProviderAccountTestResponse>("provider-account-test.response.json");
    body::<ProviderPlacementRequest>("provider-account-status.request.json");
    body::<ProviderAccountStatusResponse>("provider-account-status.response.json");
    body::<AccountGrantRequest>("account-grant-create.request.json");
    body::<MutationResponse>("account-grant-create.response.json");
    body::<AccountGrantRequest>("account-grant-delete.request.json");
    body::<MutationResponse>("account-grant-delete.response.json");
    body::<AccountGrantRequest>("account-grant-list.request.json");
    body::<AccountGrantListResponse>("account-grant-list.response.json");
    body::<RouteListRequest>("route-list.request.json");
    body::<RouteListResponse>("route-list.response.json");
    body::<RouteUpdateRequest>("route-update.request.json");
    body::<RouteUpdateResponse>("route-update.response.json");
    body::<RouteRequest>("route-delete.request.json");
    body::<MutationResponse>("route-delete.response.json");
    body::<ProviderPlacementRequest>("setup-options.request.json");
    body::<SetupOptionsResponse>("setup-options.response.json");
    body::<SetupStartRequest>("setup-start.request.json");
    body::<SetupResponse>("setup-start.response.json");
    body::<SetupStatusRequest>("setup-status.request.json");
    body::<SetupResponse>("setup-status.response.json");
    body::<SurfaceListRequest>("surface-list.request.json");
    body::<SurfaceListResponse>("surface-list.response.json");
    body::<SurfaceRequest>("surface-dismiss.request.json");
    body::<MutationResponse>("surface-dismiss.response.json");
    body::<SurfaceBindCodeCreateRequest>("surface-bind-code-create.request.json");
    body::<ChallengeCodeResponse>("surface-bind-code-create.response.json");
    body::<ActorLinkCodeCreateRequest>("actor-link-code-create.request.json");
    body::<ActorLinkChallengeResponse>("actor-link-code-create.response.json");
    body::<ActorLinkClaimRequest>("actor-link-claim-status.request.json");
    body::<ActorLinkClaimResponse>("actor-link-claim-status.response.json");
    body::<ActorLinkClaimRequest>("actor-link-claim-confirm.request.json");
    body::<ActorLinkClaimResponse>("actor-link-claim-confirm.response.json");
    body::<ActorLinkClaimEvent>("actor-link-claim-event.response.json");
    body::<ActorLinkListRequest>("actor-link-list.request.json");
    body::<ActorLinkListResponse>("actor-link-list.response.json");
    body::<ActorLinkDeleteRequest>("actor-link-delete.request.json");
    body::<MutationResponse>("actor-link-delete.response.json");
}

#[test]
fn invalid_frames_are_rejected() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/messaging-frame.v3.schema.json")).unwrap();
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
        serde_json::from_value::<ProviderAccountTestRequest>(missing["body"].clone()).is_err(),
        "Rust DTO unexpectedly accepted a missing required extension object"
    );
}

#[test]
fn target_manifest_matches_rust_inventory() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/targets.v1.json")).unwrap();
    assert_eq!(manifest["contractVersion"], env!("CARGO_PKG_VERSION"));
    for field in ["requestTargets", "responseTargets"] {
        let targets = manifest[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(targets, MANAGEMENT_TARGETS, "manifest field {field}");
    }
    let event_targets = manifest["eventTargets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(event_targets, messaging_api::EVENT_TARGETS);
}
