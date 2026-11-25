use onboard_api::authority::*;
use onboard_api::*;
use serde_json::Value;

#[test]
fn rust_and_schema_accept_the_same_canonical_corpus() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/onboard.v1.schema.json")).unwrap();
    let cases: Vec<Value> = [
        include_str!("../fixtures/conformance.json"),
        include_str!("../fixtures/authority.conformance.json"),
    ]
    .into_iter()
    .flat_map(|text| {
        let corpus: Value = serde_json::from_str(text).unwrap();
        corpus["cases"].as_array().unwrap().clone()
    })
    .collect();
    for case in &cases {
        let kind = case["kind"].as_str().unwrap();
        let value = case["value"].clone();
        let expected = case["valid"].as_bool().unwrap();
        let mut selected = schema.clone();
        selected["$ref"] = format!("#/$defs/{kind}").into();
        let validator = jsonschema::validator_for(&selected).unwrap();
        assert_eq!(
            validator.is_valid(&value),
            expected,
            "schema: {}",
            case["name"]
        );
        let accepted = match kind {
            "product" => serde_json::from_value::<Product>(value).is_ok_and(|p| {
                if p.validate().is_err() {
                    return false;
                }
                if let Some(digest) = case["expectedDigest"].as_str() {
                    assert_eq!(p.digest().unwrap(), digest);
                }
                true
            }),
            "plan" => serde_json::from_value::<Plan>(value).is_ok_and(|p| {
                p.validate(&serde_json::from_value(case["product"].clone()).unwrap())
                    .is_ok()
            }),
            "start" => serde_json::from_value::<Start>(value).is_ok_and(|s| {
                s.validate(&serde_json::from_value(case["product"].clone()).unwrap())
                    .is_ok()
            }),
            "status" => serde_json::from_value::<Status>(value).is_ok_and(|s| {
                s.validate(&serde_json::from_value(case["product"].clone()).unwrap())
                    .is_ok()
            }),
            "operationGrant" => serde_json::from_value::<OperationGrant>(value).is_ok_and(|s| {
                s.validate(
                    &serde_json::from_value(case["product"].clone()).unwrap(),
                    &serde_json::from_value(case["plan"].clone()).unwrap(),
                    case["nowMs"].as_u64().unwrap(),
                )
                .is_ok()
            }),
            "authorityChallenge" => {
                serde_json::from_value::<AuthorityChallenge>(value).is_ok_and(|s| {
                    s.validate(
                        &serde_json::from_value(case["product"].clone()).unwrap(),
                        &serde_json::from_value(case["plan"].clone()).unwrap(),
                        case["deviceNonce"].as_str().unwrap(),
                        case["nowMs"].as_u64().unwrap(),
                    )
                    .is_ok()
                })
            }
            "possessionProof" => serde_json::from_value::<PossessionProof>(value).is_ok_and(|s| {
                s.validate(case["challengeDigest"].as_str().unwrap())
                    .is_ok()
            }),
            _ => panic!("unhandled fixture kind: {kind}"),
        };
        assert_eq!(accepted, expected, "Rust: {}", case["name"]);
    }
}

fn product_and_plan() -> (Product, Plan) {
    let corpus: Value = serde_json::from_str(include_str!("../fixtures/conformance.json")).unwrap();
    let case = corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "plan-join")
        .unwrap();
    (
        serde_json::from_value(case["product"].clone()).unwrap(),
        serde_json::from_value(case["value"].clone()).unwrap(),
    )
}

#[test]
fn descriptor_order_is_irrelevant_but_identity_or_network_changes_are_not() {
    let (mut product, plan) = product_and_plan();
    product.components.reverse();
    assert!(plan.validate(&product).is_ok());
    product.network.ble = !product.network.ble;
    assert!(plan.validate(&product).is_err());
}

#[test]
fn stale_installations_and_first_hub_promotion_without_host_are_rejected() {
    let (mut product, mut plan) = product_and_plan();
    plan.generation = "0".repeat(32);
    assert!(plan.validate(&product).is_err());
    product.can_host_hub = false;
    product.components.retain(|c| c.role != Role::Host);
    plan.generation = product.generation.clone();
    plan.descriptor_digest = product.digest().unwrap();
    plan.hub = Hub::First;
    assert!(plan.validate(&product).is_err());
}

#[test]
fn status_cannot_hide_missing_components_or_convert_pending_work_to_success() {
    let (product, plan) = product_and_plan();
    let mut status = Status {
        plan,
        phase: Phase::Complete,
        state: State::Complete,
        components: product
            .components
            .iter()
            .map(|c| ComponentResult {
                role: c.role,
                state: ComponentState::Confirmed,
            })
            .collect(),
        reason: None,
        updated_at_ms: 1_795_000_000_123,
    };
    assert!(status.validate(&product).is_ok());
    status.components[2].state = ComponentState::Pending;
    assert!(status.validate(&product).is_err());
    status.components.pop();
    assert!(status.validate(&product).is_err());
    status.components = product
        .components
        .iter()
        .map(|c| ComponentResult {
            role: c.role,
            state: ComponentState::Confirmed,
        })
        .collect();
    status.updated_at_ms = MAX_SAFE_TIMESTAMP + 1;
    assert!(status.validate(&product).is_err());
}
