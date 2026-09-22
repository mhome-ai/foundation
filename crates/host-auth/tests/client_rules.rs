use core_api::host::auth::{AuthError, ErrorCode, HostContext, PROTOCOL};
use host_auth::{session::*, signatures::*};
use http::{HeaderMap, StatusCode};

fn context(host: &str, boot: &str) -> HostContext {
    HostContext {
        protocol: PROTOCOL.into(),
        host_id: host.into(),
        boot_id: boot.into(),
        probe_id: "probe".into(),
        server_time: 100,
    }
}

#[test]
fn context_must_match_trusted_host_and_fresh_probe() {
    let host = new_key();
    let key = public_key(host.verifying_key());
    let proof = sign_jws(&host, "meow-host-context", &context("a", "boot")).unwrap();
    assert!(verify_context(&proof, &key, "a", "probe").is_ok());
    assert!(verify_context(&proof, &key, "b", "probe").is_err());
    assert!(verify_context(&proof, &key, "a", "another").is_err());
    assert!(verify_context(&proof, &public_key(new_key().verifying_key()), "a", "probe").is_err());
}

#[test]
fn caches_isolate_hosts_and_retired_identities() {
    let mut cache = Sessions::default();
    let a = cache.publish("a", 0, context("a", "boot"), false).unwrap();
    let b = cache.publish("b", 0, context("b", "boot"), true).unwrap();
    assert!(!b.satisfies(&a, Refresh::Enrollment));
    cache.retire(1);
    assert!(cache.get("a", 0).is_none());
    assert!(cache.publish("a", 0, context("a", "boot"), true).is_err());
    assert!(cache.publish("a", 1, context("b", "boot"), true).is_err());
    let next = cache.publish("a", 1, context("a", "boot"), true).unwrap();
    assert!(!next.satisfies(&a, Refresh::Enrollment));
}

#[test]
fn context_refresh_does_not_suppress_registration_and_reboot_resets_its_hint() {
    let mut cache = Sessions::default();
    let old = cache.publish("a", 0, context("a", "boot"), false).unwrap();
    let refreshed = cache.publish("a", 0, context("a", "boot"), false).unwrap();
    assert!(refreshed.satisfies(&old, Refresh::Context));
    assert!(!refreshed.satisfies(&old, Refresh::Enrollment));
    let enrolled = cache.publish("a", 0, context("a", "boot"), true).unwrap();
    assert!(enrolled.satisfies(&old, Refresh::Enrollment));
    let rebooted = cache
        .publish("a", 0, context("a", "next-boot"), false)
        .unwrap();
    assert!(!rebooted.satisfies(&old, Refresh::Enrollment));
}

#[test]
fn only_authenticated_bound_rejections_trigger_one_recovery() {
    let client = new_key();
    let host = new_key();
    let host_key = public_key(host.verifying_key());
    let session = Sessions::default()
        .publish("h", 0, context("h", "boot"), false)
        .unwrap();
    let url = "http://host/restart";
    let exchange = Exchange::sign(&client, "alice", &session, "POST", url, b"{}").unwrap();
    let identity = request_identity(&exchange.headers).unwrap();
    let binding = ResponseIdentity {
        host_id: "h".into(),
        request_digest: request_digest(&exchange.headers, "POST", url).unwrap(),
    };
    for (code, refresh) in [
        (ErrorCode::HostEnrollmentRequired, Refresh::Enrollment),
        (ErrorCode::HostReauthRequired, Refresh::Enrollment),
        (ErrorCode::HostContextChanged, Refresh::Context),
    ] {
        let body = serde_json::to_vec(&AuthError {
            code: code.clone(),
            host_id: "h".into(),
            boot_id: "boot".into(),
            request_id: identity.request_id.clone(),
            key_set_version: 1,
        })
        .unwrap();
        let headers = sign_response(&host, 401, &body, &binding).unwrap();
        assert_eq!(
            exchange
                .verify(&host_key, StatusCode::UNAUTHORIZED, &headers, &body, 0)
                .unwrap(),
            Outcome::Refresh(refresh)
        );
        assert_eq!(
            exchange
                .verify(&host_key, StatusCode::UNAUTHORIZED, &headers, &body, 1)
                .unwrap(),
            Outcome::Rejected(Some(code))
        );
        assert!(exchange
            .verify(
                &host_key,
                StatusCode::UNAUTHORIZED,
                &HeaderMap::new(),
                &body,
                0
            )
            .is_err());
        let other = Exchange::sign(&client, "alice", &session, "POST", url, b"{}").unwrap();
        assert!(other
            .verify(&host_key, StatusCode::UNAUTHORIZED, &headers, &body, 0)
            .is_err());
        assert!(exchange
            .verify(&host_key, StatusCode::OK, &headers, &body, 0)
            .is_err());
    }
}

#[test]
fn separate_native_and_hub_keys_stay_separate_for_the_same_user() {
    let native = new_key();
    let hub = new_key();
    let session = Sessions::default()
        .publish("h", 0, context("h", "boot"), false)
        .unwrap();
    let n = Exchange::sign(&native, "alice", &session, "GET", "http://host/info", b"").unwrap();
    let h = Exchange::sign(&hub, "alice", &session, "GET", "http://host/info", b"").unwrap();
    assert_eq!(
        request_identity(&n.headers).unwrap().user_id,
        request_identity(&h.headers).unwrap().user_id
    );
    assert_ne!(
        request_identity(&n.headers).unwrap().client_key_id,
        request_identity(&h.headers).unwrap().client_key_id
    );
    assert!(verify_request(
        &n.headers,
        hub.verifying_key(),
        "GET",
        "http://host/info",
        b""
    )
    .is_err());
}

#[test]
fn cloud_confirmed_registration_binds_host_user_client_and_challenge() {
    use core_api::host::auth::{EnrollmentChallenge, Grant};
    let host = new_key();
    let client = public_key(new_key().verifying_key());
    let challenge = EnrollmentChallenge {
        host_id: "h".into(),
        boot_id: "b".into(),
        user_id: "alice".into(),
        client_public_key: client.clone(),
        probe_id: "p".into(),
        challenge: "ticket".into(),
        expires_at: 300,
    };
    let signed = sign_jws(&host, "meow-host-enrollment-challenge", &challenge).unwrap();
    let grant = Grant {
        sub: "alice".into(),
        aud: "meow-host-client-registration".into(),
        jti: "j".into(),
        iat: 0,
        exp: 300,
        host_id: "h".into(),
        host_public_key: public_key(host.verifying_key()),
        client_public_key: Some(client.clone()),
        challenge: "ticket".into(),
        boot_id: Some("b".into()),
    };
    let cloud = new_key();
    let proof = sign_jws(&cloud, "trusted-cloud-fixture", &grant).unwrap();
    assert!(cloud_confirmed_enrollment(&proof, &signed, "h", "alice", &client, "p").is_ok());
    assert!(cloud_confirmed_enrollment(&proof, &signed, "other", "alice", &client, "p").is_err());
    assert!(cloud_confirmed_enrollment(&proof, &signed, "h", "bob", &client, "p").is_err());
    assert!(
        cloud_confirmed_enrollment(&proof, &signed, "h", "alice", &client, "wrong-probe").is_err()
    );
    let mut wrong = grant.clone();
    wrong.challenge = "other-ticket".into();
    let wrong = sign_jws(&cloud, "trusted-cloud-fixture", &wrong).unwrap();
    assert!(cloud_confirmed_enrollment(&wrong, &signed, "h", "alice", &client, "p").is_err());
    let forged = sign_jws(&new_key(), "meow-host-enrollment-challenge", &challenge).unwrap();
    assert!(cloud_confirmed_enrollment(&proof, &forged, "h", "alice", &client, "p").is_err());
}
