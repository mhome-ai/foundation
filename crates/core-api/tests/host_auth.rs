use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use core_api::host::auth::*;
fn key(kid: &str, status: SigningKeyStatus) -> SigningKey {
    SigningKey {
        kid: kid.into(),
        status,
        n: URL_SAFE_NO_PAD.encode([0x80; 256]),
        e: "AQAB".into(),
    }
}
fn set(version: u64, keys: Vec<SigningKey>) -> SigningKeySet {
    SigningKeySet { version, keys }
}
#[test]
fn authoritative_rotation_rejects_disabled_token_keys() {
    let old = set(1, vec![key("one", SigningKeyStatus::Active)]);
    let overlap = set(
        2,
        vec![
            key("one", SigningKeyStatus::VerifyOnly),
            key("two", SigningKeyStatus::Active),
        ],
    );
    overlap.validate_update(Some(&old)).unwrap();
    assert!(overlap.permits_token("one"));
    let disabled = set(
        3,
        vec![
            key("one", SigningKeyStatus::Disabled),
            key("two", SigningKeyStatus::Active),
        ],
    );
    disabled.validate_update(Some(&overlap)).unwrap();
    assert!(!disabled.permits_token("one"));
    assert!(disabled.permits_token("two"));
    assert!(old.validate_update(Some(&disabled)).is_err());
    let mut resurrect = overlap.clone();
    resurrect.version = 4;
    assert!(resurrect.validate_update(Some(&disabled)).is_err());
}
#[test]
fn same_version_is_order_independent_but_cannot_change_contents() {
    let old = set(
        2,
        vec![
            key("one", SigningKeyStatus::VerifyOnly),
            key("two", SigningKeyStatus::Active),
        ],
    );
    let mut reordered = old.clone();
    reordered.keys.reverse();
    reordered.validate_update(Some(&old)).unwrap();
    reordered.keys[1].status = SigningKeyStatus::Disabled;
    assert!(reordered.validate_update(Some(&old)).is_err());
}
#[test]
fn missing_and_disabled_keys_cannot_authorize_clients() {
    let old = set(1, vec![key("one", SigningKeyStatus::Active)]);
    let next = set(2, vec![key("two", SigningKeyStatus::Active)]);
    next.validate_update(Some(&old)).unwrap();
    assert!(!next.permits_token("one"));
    let tombstones = set(
        3,
        vec![
            key("two", SigningKeyStatus::Active),
            key("one", SigningKeyStatus::Disabled),
        ],
    );
    assert!(set(4, next.keys)
        .validate_update(Some(&tombstones))
        .is_err());
}
#[test]
fn malformed_or_rebound_keys_fail_before_cache_update() {
    let old = set(1, vec![key("one", SigningKeyStatus::Active)]);
    for change in 0..6 {
        let mut next = old.clone();
        next.version = 2;
        match change {
            0 => next.keys[0].kid.clear(),
            1 => next.keys.push(next.keys[0].clone()),
            2 => next.keys[0].e = "Aw".into(),
            3 => next.keys[0].n = URL_SAFE_NO_PAD.encode([0x81; 256]),
            4 => next.keys[0].status = SigningKeyStatus::VerifyOnly,
            _ => next.version = 0,
        }
        assert!(next.validate_update(Some(&old)).is_err());
    }
}
#[test]
fn claim_wire_requires_environment_exchange_and_rejects_old_protocol_fields() {
    use core_api::host::access::ClaimRequest;
    let wire = serde_json::json!({"env": "prod", "proof": "grant", "exchangeKey": "ephemeral", "claimToken": null});
    let request: ClaimRequest = serde_json::from_value(wire.clone()).unwrap();
    assert!(request.claim_token.is_none());
    assert_eq!(serde_json::to_value(request).unwrap(), wire);
    for field in ["env", "exchangeKey"] {
        let mut missing = wire.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(serde_json::from_value::<ClaimRequest>(missing).is_err());
    }
    let mut old = wire.clone();
    old["clientPublicKey"] = "old-key".into();
    assert!(serde_json::from_value::<ClaimRequest>(old).is_err());
    let mut bluetooth = wire;
    bluetooth["claimToken"] = "token".into();
    assert_eq!(
        serde_json::from_value::<ClaimRequest>(bluetooth)
            .unwrap()
            .claim_token
            .as_deref(),
        Some("token")
    );
    assert_eq!(CLAIM_TOKEN_REQUIRED, "HOST_CLAIM_TOKEN_REQUIRED");
}
