use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub(super) fn verify(
    value: &Value,
    peer: &PublicKey,
    candidate: &Candidate,
    nonce: &str,
) -> Result<(), String> {
    let env: RuntimeEnv =
        serde_json::from_value(value.get("env").cloned().unwrap_or(json!("prod")))
            .map_err(|e| e.to_string())?;
    let policy: RuntimeSpacePolicy =
        serde_json::from_value(value["runtimeSpacePolicy"].clone()).map_err(|e| e.to_string())?;
    policy.validate().map_err(str::to_owned)?;
    let host_key: PublicKey =
        serde_json::from_value(value["hostPublicKey"].clone()).map_err(|e| e.to_string())?;
    if env != RuntimeEnv::current()
        || policy != candidate.runtime_space_policy
        || host_key != *peer
        || value["nonce"] != nonce
        || value["hostId"] != candidate.host_id
        || value["fingerprint"] != candidate.fingerprint
        || value["publicKey"] != candidate.public_key
    {
        return Err(
            "Hub challenge does not match the selected device, Space policy or TLS peer".into(),
        );
    }
    let bytes = STANDARD
        .decode(&candidate.public_key)
        .map_err(|e| e.to_string())?;
    let key = VerifyingKey::from_bytes(&bytes.try_into().map_err(|_| "Invalid Hub public key")?)
        .map_err(|e| e.to_string())?;
    for (name, payload) in [
        ("signature", nonce.as_bytes().to_vec()),
        (
            "hostTlsSignature",
            core_api::hub_commission_tls_payload(
                nonce,
                &candidate.host_id,
                &host_key,
                env,
                &policy,
            ),
        ),
    ] {
        let signature = Signature::from_slice(
            &STANDARD
                .decode(text(value, name)?)
                .map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        key.verify(&payload, &signature)
            .map_err(|_| "Invalid Hub identity proof".to_owned())?;
    }
    Ok(())
}
