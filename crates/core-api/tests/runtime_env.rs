use core_api::{
    hub_commission_tls_payload, hub_connection_proof_signing_payload, HubConnectionProofRequest,
    RuntimeEnv,
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
fn first_pairing_binds_environment_and_runtime_space() {
    use core_api::runtime_space::{RuntimeSpacePolicy, SpaceIdentity};
    let key = core_api::host::auth::PublicKey {
        x: "x".into(),
        y: "y".into(),
    };
    let multi = RuntimeSpacePolicy::Multi {};
    let single = RuntimeSpacePolicy::Single {
        space: SpaceIdentity {
            tenant_id: "tenant".into(),
            scope_id: "space".into(),
        },
    };
    let prod = hub_commission_tls_payload("nonce", "host", &key, RuntimeEnv::Prod, &multi);
    let dev = hub_commission_tls_payload("nonce", "host", &key, RuntimeEnv::Dev, &multi);
    let bound = hub_commission_tls_payload("nonce", "host", &key, RuntimeEnv::Prod, &single);
    assert_ne!(prod, dev);
    assert_ne!(prod, bound);
    assert_ne!(
        bound,
        hub_commission_tls_payload("nonce", "host", &key, RuntimeEnv::E2e, &single)
    );
    assert!(std::str::from_utf8(&prod).is_err());
}
