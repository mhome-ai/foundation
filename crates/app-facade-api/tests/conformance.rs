use app_facade_api::hub::{HubState, LocalHubState, APP_TARGETS, LOCAL_TARGETS};
use app_facade_api::messaging::{
    ActorLinkChallengeResponse, ActorLinkClaimEvent, ActorLinkClaimRequest, ActorLinkClaimResponse,
    ActorLinkCodeCreateRequest, ActorLinkDeleteRequest, ActorLinkListRequest,
    ActorLinkListResponse, ChallengeCodeResponse, MutationResponse, ProviderAccountListResponse,
    ProviderAccountRequest, ProviderAccountStatusResponse, ProviderAccountTestRequest,
    ProviderAccountTestResponse, ProviderAccountUpdateRequest, ProviderAccountUpdateResponse,
    ProviderListRequest, ProviderListResponse, ProviderPlacementRequest, RouteListRequest,
    RouteListResponse, RouteRequest, RouteUpdateRequest, RouteUpdateResponse, SetupOptionsResponse,
    SetupResponse, SetupStartRequest, SetupStatusRequest, SharedAccountGrantListRequest,
    SharedAccountGrantListResponse, SharedAccountGrantRequest, SurfaceBindCodeCreateRequest,
    SurfaceListRequest, SurfaceListResponse, SurfaceRequest, MANAGEMENT_TARGETS,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::BTreeSet;

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
        "shared-account-grant-create.request.json",
        include_str!("../fixtures/shared-account-grant-create.request.json"),
    ),
    (
        "shared-account-grant-create.response.json",
        include_str!("../fixtures/shared-account-grant-create.response.json"),
    ),
    (
        "shared-account-grant-delete.request.json",
        include_str!("../fixtures/shared-account-grant-delete.request.json"),
    ),
    (
        "shared-account-grant-delete.response.json",
        include_str!("../fixtures/shared-account-grant-delete.response.json"),
    ),
    (
        "shared-account-grant-list.request.json",
        include_str!("../fixtures/shared-account-grant-list.request.json"),
    ),
    (
        "shared-account-grant-list.response.json",
        include_str!("../fixtures/shared-account-grant-list.response.json"),
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
fn hub_manifests_and_fixtures_match_the_shared_contract() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/hub-targets.v1.json")).unwrap();
    let strings = |field: &str| {
        manifest[field]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>()
    };
    assert_eq!(strings("appTargets"), APP_TARGETS);
    assert_eq!(strings("localTargets"), LOCAL_TARGETS);
    assert_eq!(strings("appInvocationModes"), ["direct"]);
    assert_eq!(manifest["localPayload"], "domainInput");
    assert_eq!(manifest["localForwardable"], false);

    let configured: HubState =
        serde_json::from_str(include_str!("../fixtures/hub-state.configured.json")).unwrap();
    configured.validate(None).unwrap();
    let revoked: HubState =
        serde_json::from_str(include_str!("../fixtures/hub-state.revoked.json")).unwrap();
    revoked.validate(Some("hub-old")).unwrap();
    let local: LocalHubState =
        serde_json::from_str(include_str!("../fixtures/local-hub-state.configured.json")).unwrap();
    local.validate(None).unwrap();

    let schema: Value =
        serde_json::from_str(include_str!("../schema/hub-state.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for fixture in [
        include_str!("../fixtures/hub-state.configured.json"),
        include_str!("../fixtures/hub-state.revoked.json"),
    ] {
        let value: Value = serde_json::from_str(fixture).unwrap();
        assert!(validator.is_valid(&value));
    }

    let local_schema: Value =
        serde_json::from_str(include_str!("../schema/local-hub-state.v1.schema.json")).unwrap();
    let local_validator = jsonschema::options()
        .with_resource(
            "https://schemas.mhome.ai/app-facade/hub-state.v1.schema.json",
            jsonschema::Resource::from_contents(schema).unwrap(),
        )
        .build(&local_schema)
        .unwrap();
    let local_value: Value =
        serde_json::from_str(include_str!("../fixtures/local-hub-state.configured.json")).unwrap();
    assert!(local_validator.is_valid(&local_value));
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
    body::<SharedAccountGrantRequest>("shared-account-grant-create.request.json");
    body::<MutationResponse>("shared-account-grant-create.response.json");
    body::<SharedAccountGrantRequest>("shared-account-grant-delete.request.json");
    body::<MutationResponse>("shared-account-grant-delete.response.json");
    body::<SharedAccountGrantListRequest>("shared-account-grant-list.request.json");
    body::<SharedAccountGrantListResponse>("shared-account-grant-list.response.json");
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
fn facade_call_credentials_are_mode_specific() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/facade-call.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for call in [
        serde_json::json!({"control": {"mode": "direct"}, "input": {}}),
        serde_json::json!({"control": {"mode": "prepare"}, "input": {}}),
        serde_json::json!({
            "control": {
                "mode": "commit",
                "preparedActionId": "action-1",
                "approvalToken": "token-1"
            },
            "input": {}
        }),
        serde_json::json!({
            "control": {"mode": "reject", "preparedActionId": "action-1"},
            "input": {}
        }),
    ] {
        assert!(validator.is_valid(&call), "valid call was rejected: {call}");
    }

    for call in [
        serde_json::json!({
            "control": {"mode": "direct", "preparedActionId": "action-1"},
            "input": {}
        }),
        serde_json::json!({
            "control": {"mode": "prepare", "approvalToken": "token-1"},
            "input": {}
        }),
        serde_json::json!({
            "control": {"mode": "commit", "preparedActionId": "action-1"},
            "input": {}
        }),
        serde_json::json!({
            "control": {
                "mode": "reject",
                "preparedActionId": "action-1",
                "approvalToken": "token-1"
            },
            "input": {}
        }),
        serde_json::json!({
            "control": {"mode": "reject", "preparedActionId": "  "},
            "input": {}
        }),
    ] {
        assert!(
            !validator.is_valid(&call),
            "invalid call unexpectedly passed: {call}"
        );
    }
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
    assert_eq!(event_targets, app_facade_api::messaging::EVENT_TARGETS);

    for target in MANAGEMENT_TARGETS {
        assert_eq!(
            app_facade_api::messaging::required_management_operation(target).is_some(),
            *target != app_facade_api::messaging::PROVIDER_LIST_TARGET,
            "management capability mapping for {target}"
        );
    }
}

#[test]
fn routing_manifest_matches_rust_policy() {
    use app_facade_api::routing::{
        route_policy_for_target, ExecutionSelector, RelayPolicy, CLOUD_RELAY_TARGETS, CLOUD_TARGETS,
    };

    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/routing.v1.json")).unwrap();
    let schema: Value =
        serde_json::from_str(include_str!("../schema/routing.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(
        validator.is_valid(&manifest),
        "routing manifest does not conform to its public schema"
    );
    let rules = &manifest["rules"];
    let strings = |field: &str| {
        rules[field]
            .as_array()
            .unwrap_or_else(|| panic!("routing rules.{field} must be an array"))
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<BTreeSet<_>>()
    };

    assert_eq!(manifest["contractVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["targetPrefix"], "/app/");
    assert_eq!(manifest["default"]["execution"], "scopeMode");
    assert_eq!(manifest["default"]["relay"], "directOnly");
    assert_eq!(manifest["matching"], "exactBeforePrefix");
    assert_eq!(manifest["placementJsonPointer"], "/input/placement");

    let expected_rule_groups = [
        "cloudPrefixes",
        "cloudTargets",
        "scopeModeTargets",
        "hubPrefixes",
        "requestPlacementPrefixes",
        "cloudRelayTargets",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    assert_eq!(
        rules
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        expected_rule_groups
    );

    let exact_groups = ["cloudTargets", "scopeModeTargets", "cloudRelayTargets"];
    let mut exact_owners = std::collections::BTreeMap::new();
    for field in exact_groups {
        for target in strings(field) {
            assert!(
                exact_owners.insert(target.clone(), field).is_none(),
                "exact route {target} belongs to more than one rule group"
            );
        }
    }

    let prefix_groups = ["cloudPrefixes", "hubPrefixes", "requestPlacementPrefixes"];
    let prefixes = prefix_groups
        .into_iter()
        .flat_map(|field| {
            strings(field)
                .into_iter()
                .map(move |prefix| (field, prefix))
        })
        .collect::<Vec<_>>();
    for (index, (left_field, left)) in prefixes.iter().enumerate() {
        for (right_field, right) in prefixes.iter().skip(index + 1) {
            assert!(
                !left.starts_with(right.as_str()) && !right.starts_with(left.as_str()),
                "routing prefixes {left} ({left_field}) and {right} ({right_field}) overlap"
            );
        }
    }
    assert_eq!(
        strings("cloudTargets"),
        CLOUD_TARGETS
            .iter()
            .map(|target| (*target).to_owned())
            .collect()
    );
    assert_eq!(
        strings("cloudRelayTargets"),
        CLOUD_RELAY_TARGETS
            .iter()
            .map(|target| (*target).to_owned())
            .collect()
    );

    for target in strings("cloudTargets") {
        let policy = route_policy_for_target(&target).unwrap();
        assert_eq!(policy.execution, ExecutionSelector::Cloud, "{target}");
        assert_eq!(policy.relay, RelayPolicy::DirectOnly, "{target}");
    }
    for target in strings("scopeModeTargets") {
        let policy = route_policy_for_target(&target).unwrap();
        assert_eq!(policy.execution, ExecutionSelector::ScopeMode, "{target}");
    }
    for target in strings("cloudRelayTargets") {
        let policy = route_policy_for_target(&target).unwrap();
        assert_eq!(policy.execution, ExecutionSelector::Hub, "{target}");
        assert_eq!(policy.relay, RelayPolicy::CloudRelayAllowed, "{target}");
    }
    for prefix in strings("cloudPrefixes") {
        let policy = route_policy_for_target(&format!("{prefix}conformance-probe")).unwrap();
        assert_eq!(policy.execution, ExecutionSelector::Cloud, "{prefix}");
    }
    for prefix in strings("hubPrefixes") {
        let policy = route_policy_for_target(&format!("{prefix}conformance-probe")).unwrap();
        assert_eq!(policy.execution, ExecutionSelector::Hub, "{prefix}");
    }
    for prefix in strings("requestPlacementPrefixes") {
        let policy = route_policy_for_target(&format!("{prefix}conformance-probe")).unwrap();
        assert_eq!(
            policy.execution,
            ExecutionSelector::RequestPlacement,
            "{prefix}"
        );
    }
}

#[test]
fn topology_snapshot_matches_public_schema() {
    use app_facade_api::topology::{
        TopologyDurability, TopologyEdge, TopologyEntity, TopologyObservation,
        TopologyObservationState, TopologyRelation, TopologyRelationBasis, TopologyRouteKind,
        TopologyRuntime, TopologyRuntimeState, TopologySnapshot, CONTRACT,
    };

    let snapshot = TopologySnapshot {
        contract: CONTRACT.to_string(),
        scope_id: "space-1".to_string(),
        generation: "generation-1".to_string(),
        revision: 1,
        observed_at_ms: 1,
        entities: vec![
            TopologyEntity::Host {
                id: "host:machine".to_string(),
                host_id: "machine".to_string(),
                display_name: "Machine".to_string(),
            },
            TopologyEntity::Hub {
                id: "hub:h".to_string(),
                hub_id: "h".to_string(),
                host_id: "machine".to_string(),
                display_name: "Hub".to_string(),
            },
            TopologyEntity::Node {
                id: "node:camera".to_string(),
                node_id: "camera".to_string(),
                node_type: "camera".to_string(),
                host_id: "machine".to_string(),
                display_name: "Front Door Camera".to_string(),
                binding_state: "active".to_string(),
                runtime: TopologyRuntime {
                    state: TopologyRuntimeState::Healthy,
                    reason: None,
                    updated_at_ms: Some(1),
                },
            },
            TopologyEntity::Client {
                id: "client:a:desktop".to_string(),
                app_client_id: "a:desktop".to_string(),
                cur_client_id: Some("L:socket".to_string()),
                source: "desktop".to_string(),
                device_type: "desktop".to_string(),
                display_name: "Living Room Mac".to_string(),
                is_current: true,
                route_kind: Some(TopologyRouteKind::CloudBridgeShadow),
                has_local_credential: true,
            },
            TopologyEntity::Cloud {
                id: "cloud:primary".to_string(),
                display_name: "Cloud".to_string(),
            },
        ],
        edges: vec![
            TopologyEdge {
                id: "hub:h->cloud:primary".to_string(),
                source: "hub:h".to_string(),
                target: "cloud:primary".to_string(),
                relation: TopologyRelation::HubCloud,
                basis: TopologyRelationBasis::HubCommission,
                durability: TopologyDurability::Durable,
                observation: TopologyObservation {
                    state: TopologyObservationState::Disconnected,
                    observed_at_ms: 1,
                    reason: Some("offline".to_string()),
                },
            },
            TopologyEdge {
                id: "node:camera->hub:h".to_string(),
                source: "node:camera".to_string(),
                target: "hub:h".to_string(),
                relation: TopologyRelation::NodeHub,
                basis: TopologyRelationBasis::NodeBinding,
                durability: TopologyDurability::Durable,
                observation: TopologyObservation {
                    state: TopologyObservationState::Connected,
                    observed_at_ms: 1,
                    reason: None,
                },
            },
            TopologyEdge {
                id: "client:a:desktop->hub:h".to_string(),
                source: "client:a:desktop".to_string(),
                target: "hub:h".to_string(),
                relation: TopologyRelation::ClientHub,
                basis: TopologyRelationBasis::LocalCredential,
                durability: TopologyDurability::Durable,
                observation: TopologyObservation {
                    state: TopologyObservationState::Disconnected,
                    observed_at_ms: 1,
                    reason: Some("no local route".to_string()),
                },
            },
            TopologyEdge {
                id: "client:a:desktop->cloud:primary".to_string(),
                source: "client:a:desktop".to_string(),
                target: "cloud:primary".to_string(),
                relation: TopologyRelation::ClientCloud,
                basis: TopologyRelationBasis::LiveRoute,
                durability: TopologyDurability::Ephemeral,
                observation: TopologyObservation {
                    state: TopologyObservationState::Connected,
                    observed_at_ms: 1,
                    reason: None,
                },
            },
        ],
    };
    let schema: Value =
        serde_json::from_str(include_str!("../schema/topology.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::to_value(snapshot).unwrap()));
}

#[test]
fn device_topology_snapshots_match_public_schema_and_graph_invariants() {
    use app_facade_api::device_topology::DeviceTopologySnapshot;

    let schema: Value =
        serde_json::from_str(include_str!("../schema/device-topology.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for fixture in [
        include_str!("../fixtures/device-topology.snapshot.json"),
        include_str!("../fixtures/device-topology.partial.json"),
    ] {
        let value: Value = serde_json::from_str(fixture).unwrap();
        let errors = validator
            .iter_errors(&value)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(
            errors.is_empty(),
            "Device Topology fixture failed: {errors:?}"
        );

        let snapshot: DeviceTopologySnapshot = serde_json::from_value(value).unwrap();
        snapshot.validate().unwrap();
    }

    let complete: Value =
        serde_json::from_str(include_str!("../fixtures/device-topology.snapshot.json")).unwrap();
    let relations = complete["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["relation"].as_str().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        relations,
        BTreeSet::from([
            "integration-provider",
            "plugin-provider",
            "plugin-connection",
            "provider-connection",
            "provider-device",
            "connection-device",
            "connection-source",
            "source-device",
        ])
    );

    let partial: Value =
        serde_json::from_str(include_str!("../fixtures/device-topology.partial.json")).unwrap();
    assert!(partial["entities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entity| entity["kind"] == "pluginInstance"));
    for relation in ["plugin-provider", "plugin-connection"] {
        let edge = partial["edges"]
            .as_array()
            .unwrap()
            .iter()
            .find(|edge| edge["relation"] == relation)
            .unwrap();
        assert_eq!(edge["durability"], "durable");
        assert_eq!(edge["observation"]["state"], "disconnected");
    }

    let mut missing_device_integration = complete.clone();
    missing_device_integration["entities"]
        .as_array_mut()
        .unwrap()
        .retain(|entity| entity["kind"] != "deviceIntegration");

    let mut multiple_device_integrations = complete.clone();
    let device_integration = multiple_device_integrations["entities"][0].clone();
    multiple_device_integrations["entities"]
        .as_array_mut()
        .unwrap()
        .push(device_integration);

    let mut complete_with_issues = complete.clone();
    complete_with_issues["issues"] = serde_json::json!([{
        "code": "unexpectedPartialFailure",
        "message": "A complete snapshot cannot carry partial failure details"
    }]);

    let mut partial_without_issues = complete;
    partial_without_issues["completeness"] = serde_json::json!("partial");

    let mut invalid_relation_basis: Value =
        serde_json::from_str(include_str!("../fixtures/device-topology.snapshot.json")).unwrap();
    invalid_relation_basis["edges"][0]["basis"] = serde_json::json!("twinSource");

    let mut empty_provider_mode: Value =
        serde_json::from_str(include_str!("../fixtures/device-topology.snapshot.json")).unwrap();
    let provider = empty_provider_mode["entities"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entity| entity["kind"] == "provider")
        .unwrap();
    provider["mode"] = serde_json::json!("");

    for invalid in [
        missing_device_integration,
        multiple_device_integrations,
        complete_with_issues,
        partial_without_issues,
        invalid_relation_basis,
        empty_provider_mode,
    ] {
        assert!(
            !validator.is_valid(&invalid),
            "invalid Device Topology snapshot unexpectedly matched the public schema: {invalid}"
        );
    }
}

#[test]
fn device_topology_is_registered_in_the_app_facade_manifest() {
    use app_facade_api::device_topology::{CONTRACT, GET_TARGET};

    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    assert_eq!(manifest["contractVersion"], env!("CARGO_PKG_VERSION"));

    let domain = &manifest["domains"]["deviceTopology"];
    assert_eq!(domain["contract"], CONTRACT);
    assert_eq!(domain["schema"], "schema/device-topology.v1.schema.json");
    assert_eq!(domain["requestTargets"], serde_json::json!([GET_TARGET]));
    assert_eq!(domain["eventTargets"], serde_json::json!([]));
}

#[test]
fn artifact_is_registered_as_a_scope_mode_app_facade_domain() {
    use app_facade_api::artifact::{REQUEST_TARGETS, RESOLVE_TARGET};
    use app_facade_api::routing::{route_policy_for_target, ExecutionSelector, RelayPolicy};

    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    let domain = &manifest["domains"]["artifact"];
    assert_eq!(domain["contract"], "mhome.artifact.v1");
    assert_eq!(
        domain["requestTargets"],
        serde_json::to_value(REQUEST_TARGETS).unwrap()
    );
    assert_eq!(RESOLVE_TARGET, "/app/artifact/resolve");
    let policy = route_policy_for_target(RESOLVE_TARGET).unwrap();
    assert_eq!(policy.execution, ExecutionSelector::ScopeMode);
    assert_eq!(policy.relay, RelayPolicy::DirectOnly);
}
