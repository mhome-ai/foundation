use core_api::{
    hub_commission_tls_payload, hub_commission_tls_payload_for_env,
    hub_connection_proof_signing_payload, HubConnectionProofRequest, RuntimeEnv,
};

#[test]
fn production_proof_keeps_the_legacy_wire_and_signing_vector() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/runtime-env-proof.json")).unwrap();
    let request: HubConnectionProofRequest =
        serde_json::from_value(fixture["request"].clone()).unwrap();
    assert_eq!(request.env, RuntimeEnv::Prod);
    assert_eq!(serde_json::to_value(&request).unwrap(), fixture["request"]);
    let hex = hub_connection_proof_signing_payload(&request)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert_eq!(hex, fixture["prodSigningPayloadHex"]);
    for invalid in [serde_json::Value::Null, "".into(), "test".into(), 1.into()] {
        let mut wire = fixture["request"].clone();
        wire["env"] = invalid;
        assert!(serde_json::from_value::<HubConnectionProofRequest>(wire).is_err());
    }
}

#[test]
fn non_prod_proofs_bind_the_environment_and_cannot_reuse_prod_bytes() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../fixtures/runtime-env-proof.json")).unwrap();
    let mut request: HubConnectionProofRequest =
        serde_json::from_value(fixture["request"].clone()).unwrap();
    let prod = hub_connection_proof_signing_payload(&request);
    request.env = RuntimeEnv::Dev;
    let dev = hub_connection_proof_signing_payload(&request);
    assert_eq!(serde_json::to_value(&request).unwrap()["env"], "dev");
    request.env = RuntimeEnv::E2e;
    let e2e = hub_connection_proof_signing_payload(&request);
    assert_ne!(dev, prod);
    assert_ne!(dev, e2e);
    assert_ne!(e2e, prod);
}

#[test]
fn first_pairing_keeps_prod_tls_proof_and_binds_non_prod_env() {
    let key = core_api::host::auth::PublicKey {
        x: "x".into(),
        y: "y".into(),
    };
    let old = hub_commission_tls_payload("nonce", "host", &key);
    assert_eq!(
        old,
        hub_commission_tls_payload_for_env("nonce", "host", &key, RuntimeEnv::Prod)
    );
    let dev = hub_commission_tls_payload_for_env("nonce", "host", &key, RuntimeEnv::Dev);
    let e2e = hub_commission_tls_payload_for_env("nonce", "host", &key, RuntimeEnv::E2e);
    assert_ne!(old, dev);
    assert_ne!(dev, e2e);
}
