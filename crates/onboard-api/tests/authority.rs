//! Claim validation tests, not signature or installed-product acceptance.
use onboard_api::{authority::*, *};
use serde_json::Value;

const NOW: u64 = 1_795_000_000_123;

fn context() -> (Product, Plan, OperationGrant, AuthorityChallenge) {
    let corpus: Value = serde_json::from_str(include_str!("../fixtures/conformance.json")).unwrap();
    let cases = corpus["cases"].as_array().unwrap();
    let grant = cases
        .iter()
        .find(|v| v["name"] == "operationGrant-valid")
        .unwrap();
    let challenge = cases
        .iter()
        .find(|v| v["name"] == "authorityChallenge-valid")
        .unwrap();
    (
        serde_json::from_value(grant["product"].clone()).unwrap(),
        serde_json::from_value(grant["plan"].clone()).unwrap(),
        serde_json::from_value(grant["value"].clone()).unwrap(),
        serde_json::from_value(challenge["value"].clone()).unwrap(),
    )
}

#[test]
fn grants_cannot_change_any_approved_identity_or_plan_field() {
    let (product, plan, grant, _) = context();
    assert!(grant.validate(&product, &plan, NOW).is_ok());
    for index in 0..16 {
        let mut bad = grant.clone();
        match index {
            0 => bad.iss = "meow-host-authority".into(),
            1 => bad.aud = "meow-host-management".into(),
            2 => bad.sub.clear(),
            3 => bad.jti = "not-a-grant-id".into(),
            4 => bad.environment = Environment::Prod,
            5 => bad.product.id.push('x'),
            6 => bad.product.key_id = format!("p256:{}", "0".repeat(64)),
            7 => bad.plan.operation_id = "0".repeat(32),
            8 => bad.plan.generation = "0".repeat(32),
            9 => bad.plan.descriptor_digest = "0".repeat(64),
            10 => bad.plan.space.tenant_id.push('x'),
            11 => bad.plan.space.space_id.push('x'),
            12 => bad.plan.hub = Hub::First,
            13 => bad.protocol_version = 2,
            14 => {
                if let Hub::Join { hub_id, .. } = &mut bad.plan.hub {
                    hub_id.push('x');
                }
            }
            _ => {
                if let Hub::Join { key_id, .. } = &mut bad.plan.hub {
                    *key_id = format!("p256:{}", "0".repeat(64));
                }
            }
        }
        assert!(
            bad.validate(&product, &plan, NOW).is_err(),
            "mutation {index}"
        );
    }
    let mut changed_product = product.clone();
    changed_product.components.pop();
    assert!(grant.validate(&changed_product, &plan, NOW).is_err());
}

#[test]
fn grant_time_is_bounded_epoch_milliseconds_without_seconds_fallback() {
    let (product, plan, grant, _) = context();
    for (issued, expires, now) in [
        (0, NOW + 1000, NOW),
        (NOW + MAX_FUTURE_SKEW_MS + 1, NOW + 10_000, NOW),
        (NOW, NOW, NOW),
        (NOW, NOW + MAX_GRANT_LIFETIME_MS + 1, NOW),
        (NOW, MAX_SAFE_TIMESTAMP + 1, NOW),
        (NOW, NOW + 1000, 0),
        (NOW, NOW + 1000, MAX_SAFE_TIMESTAMP + 1),
        (NOW / 1000, NOW / 1000 + 300, NOW),
    ] {
        let mut bad = grant.clone();
        bad.issued_at_ms = issued;
        bad.expires_at_ms = expires;
        assert!(bad.validate(&product, &plan, now).is_err());
    }
    let mut exact_limit = grant;
    exact_limit.expires_at_ms = NOW + MAX_GRANT_LIFETIME_MS;
    assert!(exact_limit.validate(&product, &plan, NOW).is_ok());
    let mut small_skew = exact_limit.clone();
    small_skew.issued_at_ms = NOW + MAX_FUTURE_SKEW_MS;
    assert!(small_skew.validate(&product, &plan, NOW).is_ok());
    assert!(exact_limit
        .validate(&product, &plan, exact_limit.expires_at_ms)
        .is_err());
}

#[test]
fn renewal_challenge_is_bound_to_the_live_device_nonce_and_exact_operation() {
    let (product, plan, _, challenge) = context();
    let nonce = challenge.device_nonce.clone();
    assert!(challenge.validate(&product, &plan, &nonce, NOW).is_ok());
    assert!(challenge
        .validate(&product, &plan, &"0".repeat(32), NOW)
        .is_err());
    for index in 0..10 {
        let mut bad = challenge.clone();
        match index {
            0 => bad.nonce.clear(),
            1 => bad.product.id.push('x'),
            2 => bad.plan.operation_id = "0".repeat(32),
            3 => bad.plan.space.space_id.push('x'),
            4 => bad.environment = Environment::Prod,
            5 => bad.iss = "another-issuer".into(),
            6 => bad.aud = "another-audience".into(),
            7 => bad.protocol_version = 2,
            8 => bad.expires_at_ms = NOW + MAX_CHALLENGE_LIFETIME_MS + 1,
            _ => bad.issued_at_ms = NOW + MAX_FUTURE_SKEW_MS + 1,
        }
        assert!(
            bad.validate(&product, &plan, &nonce, NOW).is_err(),
            "mutation {index}"
        );
    }
    assert!(challenge
        .validate(&product, &plan, &nonce, challenge.expires_at_ms)
        .is_err());
    let proof = PossessionProof {
        protocol_version: 1,
        challenge_digest: "6".repeat(64),
    };
    assert!(proof.validate(&"6".repeat(64)).is_ok());
    assert!(proof.validate(&"7".repeat(64)).is_err());
    assert!(proof.validate("not-a-digest").is_err());
}

#[test]
fn seven_products_have_one_owner_independent_of_component_order() {
    let corpus: Value = serde_json::from_str(include_str!("../fixtures/conformance.json")).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for case in corpus["cases"].as_array().unwrap() {
        if case["kind"] != "product" || case["valid"] != true {
            continue;
        }
        let mut product: Product = serde_json::from_value(case["value"].clone()).unwrap();
        let roles: std::collections::BTreeSet<_> =
            product.components.iter().map(|c| c.role).collect();
        let owner = if roles.contains(&Role::Host) {
            Role::Host
        } else if roles.contains(&Role::Pod) {
            Role::Pod
        } else {
            Role::Wave
        };
        assert_eq!(product.operation_owner().unwrap(), owner);
        product.components.reverse();
        assert_eq!(product.operation_owner().unwrap(), owner);
        seen.insert(roles);
    }
    assert_eq!(seen.len(), 7);
}
