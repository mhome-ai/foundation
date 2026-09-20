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
fn authoritative_rotation_requires_refresh_only_for_retired_bindings() {
    let old = set(1, vec![key("one", SigningKeyStatus::Active)]);
    let overlap = set(
        2,
        vec![
            key("one", SigningKeyStatus::VerifyOnly),
            key("two", SigningKeyStatus::Active),
        ],
    );
    overlap.validate_update(Some(&old)).unwrap();
    assert!(overlap.permits_binding("one"));
    let disabled = set(
        3,
        vec![
            key("one", SigningKeyStatus::Disabled),
            key("two", SigningKeyStatus::Active),
        ],
    );
    disabled.validate_update(Some(&overlap)).unwrap();
    assert!(!disabled.permits_binding("one"));
    assert!(disabled.permits_binding("two"));
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
    assert!(!next.permits_binding("one"));
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
fn wire_reauth_code_is_specific_and_space_free() {
    let error = AuthError {
        code: ErrorCode::HostReauthRequired,
        host_id: "host".into(),
        boot_id: "boot".into(),
        request_id: "request".into(),
        key_set_version: 2,
    };
    let json = serde_json::to_value(error).unwrap();
    assert_eq!(json["code"], "HOST_REAUTH_REQUIRED");
    assert!(json.get("spaceId").is_none());
}
