use core_api::{host::access::*, RuntimeEnv};
use host_auth::{access::*, policy::HostPolicy, signatures::*};
use std::collections::BTreeSet;
fn setup() -> (HostPolicy, HostStatus, AccessClaims) {
    let key = new_key();
    let mut policy =
        HostPolicy::unclaimed("h".into(), RuntimeEnv::Prod, HostMode::Admin, random_id()).unwrap();
    policy.claim_verified("alice", None).unwrap();
    let status = policy.status(key_id(key.verifying_key()));
    let claims = AccessClaims {
        iss: CLOUD_ISSUER.into(),
        aud: ACCESS_AUDIENCE.into(),
        sub: "alice".into(),
        jti: random_id(),
        iat: 100,
        exp: 400,
        binding: AccessBinding {
            host_id: status.host_id.clone(),
            host_key_id: status.host_key_id.clone(),
            env: status.env,
            generation: status.generation.clone(),
            user_id: "alice".into(),
            space: None,
        },
        permissions: BTreeSet::from([Permission::Read, Permission::ManageRuntime]),
    };
    (policy, status, claims)
}
#[test]
fn access_is_short_lived_and_bound_to_a_user_host_and_environment() {
    let (policy, status, claims) = setup();
    assert!(validate_access(&claims, &status, &policy, Permission::Read, 100).is_ok());
    for change in 0..9 {
        let mut c = claims.clone();
        match change {
            0 => c.sub = "bob".into(),
            1 => c.binding.host_id = "other".into(),
            2 => c.binding.host_key_id = random_id(),
            3 => c.binding.env = RuntimeEnv::Dev,
            4 => c.binding.generation = random_id(),
            5 => c.exp = 401,
            6 => c.iat = 106,
            7 => c.iss = "another-issuer".into(),
            _ => c.aud = "meow-core".into(),
        }
        assert!(
            validate_access(&c, &status, &policy, Permission::Read, 100).is_err(),
            "case {change}"
        );
    }
    assert!(validate_access(&claims, &status, &policy, Permission::Read, 400).is_err());
    assert!(validate_access(&claims, &status, &policy, Permission::Reset, 100).is_err());
}
#[test]
fn reset_and_current_acl_override_a_previously_valid_token() {
    let (mut policy, status, claims) = setup();
    policy.add_verified_administrator("bob".into()).unwrap();
    policy.remove_verified_administrator("alice").unwrap();
    assert!(validate_access(&claims, &status, &policy, Permission::Read, 100).is_err());
    let mut reset =
        HostPolicy::unclaimed("h".into(), RuntimeEnv::Prod, HostMode::Admin, random_id()).unwrap();
    reset.claim_verified("alice", None).unwrap();
    assert!(validate_access(
        &claims,
        &reset.status(status.host_key_id),
        &reset,
        Permission::Read,
        100
    )
    .is_err());
}
#[test]
fn claim_approval_cannot_be_redeemed_without_the_clients_exchange_secret() {
    let key = new_key();
    let policy =
        HostPolicy::unclaimed("h".into(), RuntimeEnv::Prod, HostMode::Admin, random_id()).unwrap();
    let status = policy.status(key_id(key.verifying_key()));
    let secret = random_id();
    let hash = exchange_digest(&secret).unwrap();
    let challenge = HostChallenge {
        device_binding: None,
        status: status.clone(),
        host_name: "Host".into(),
        host_public_key: public_key(key.verifying_key()),
        probe_id: random_id(),
        exchange_key_hash: Some(hash.clone()),
        iat: 100,
        exp: 400,
    };
    let claims = ClaimClaims {
        iss: CLOUD_ISSUER.into(),
        aud: ACCESS_AUDIENCE.into(),
        sub: "alice".into(),
        jti: random_id(),
        iat: 100,
        exp: 400,
        binding: AccessBinding {
            host_id: status.host_id.clone(),
            host_key_id: status.host_key_id.clone(),
            env: status.env,
            generation: status.generation.clone(),
            user_id: "alice".into(),
            space: None,
        },
        challenge: sign_jws(&key, CHALLENGE_TYPE, &challenge).unwrap(),
        exchange_key_hash: hash,
        display_name: "Alice".into(),
        email: "alice@example.test".into(),
    };
    assert!(validate_claim(&claims, &status, &challenge, &secret, 100).is_ok());
    assert!(validate_claim(&claims, &status, &challenge, &random_id(), 100).is_err());
    assert!(validate_claim(&claims, &status, &challenge, &secret, 400).is_err());
}

#[test]
fn applicant_proof_only_binds_a_live_request_to_a_claimed_admin_host() {
    let (_, status, access) = setup();
    let claims = AdministratorApplicantClaims {
        iss: access.iss,
        aud: access.aud,
        sub: "bob".into(),
        jti: random_id(),
        iat: 100,
        exp: 700,
        binding: AccessBinding {
            user_id: "bob".into(),
            ..access.binding
        },
        display_name: "Bob".into(),
        email: "bob@example.test".into(),
    };
    assert!(validate_administrator_applicant(&claims, &status, 100).is_ok());
    assert!(validate_administrator_applicant(&claims, &status, 700).is_err());
    // A request is not an access token or a claim grant, even with the same binding.
    assert!(
        serde_json::from_value::<AccessClaims>(serde_json::to_value(&claims).unwrap()).is_err()
    );
    assert!(serde_json::from_value::<ClaimClaims>(serde_json::to_value(&claims).unwrap()).is_err());
    for change in 0..13 {
        let mut c = claims.clone();
        match change {
            0 => c.sub = "other".into(),
            1 => c.binding.host_id = "other".into(),
            2 => c.binding.host_key_id = random_id(),
            3 => c.binding.generation = random_id(),
            4 => c.binding.env = RuntimeEnv::Dev,
            5 => c.exp = 701,
            6 => c.iat = 106,
            7 => c.iss = "other".into(),
            8 => c.aud = "other".into(),
            9 => c.jti = "short".into(),
            10 => c.display_name = "Bad\nName".into(),
            11 => c.email = "x".repeat(321),
            _ => {
                c.binding.space = Some(core_api::runtime_space::SpaceIdentity {
                    tenant_id: "t".into(),
                    scope_id: "s".into(),
                })
            }
        }
        assert!(
            validate_administrator_applicant(&c, &status, 100).is_err(),
            "case {change}"
        );
    }
    let mut unclaimed = status.clone();
    unclaimed.claim = None;
    assert!(validate_administrator_applicant(&claims, &unclaimed, 100).is_err());
    let mut space_host = status;
    space_host.mode = HostMode::SpaceSelf;
    assert!(validate_administrator_applicant(&claims, &space_host, 100).is_err());
}
