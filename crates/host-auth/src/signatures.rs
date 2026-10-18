use anyhow::{Context, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use core_api::host::auth::PublicKey;
use p256::{
    ecdsa::{
        signature::{Signer, Verifier},
        Signature, SigningKey, VerifyingKey,
    },
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

pub fn random_id() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
pub fn new_key() -> SigningKey {
    SigningKey::random(&mut OsRng)
}
pub fn encode_key(key: &SigningKey) -> Result<String> {
    Ok(URL_SAFE_NO_PAD.encode(key.to_pkcs8_der()?.as_bytes()))
}
pub fn decode_key(value: &str) -> Result<SigningKey> {
    Ok(SigningKey::from_pkcs8_der(&URL_SAFE_NO_PAD.decode(value)?)?)
}
pub fn public_key(key: &VerifyingKey) -> PublicKey {
    let point = key.to_encoded_point(false);
    PublicKey {
        x: URL_SAFE_NO_PAD.encode(point.x().unwrap()),
        y: URL_SAFE_NO_PAD.encode(point.y().unwrap()),
    }
}
pub fn verifying_key(key: &PublicKey) -> Result<VerifyingKey> {
    let x = URL_SAFE_NO_PAD.decode(&key.x)?;
    let y = URL_SAFE_NO_PAD.decode(&key.y)?;
    anyhow::ensure!(x.len() == 32 && y.len() == 32, "invalid P-256 public key");
    let mut bytes = vec![4];
    bytes.extend(x);
    bytes.extend(y);
    Ok(VerifyingKey::from_sec1_bytes(&bytes)?)
}
pub fn key_id(key: &VerifyingKey) -> String {
    // RFC 7638 P-256 JWK thumbprint, canonical member order.
    let key = public_key(key);
    let canonical = format!(
        "{{\"crv\":\"P-256\",\"kty\":\"EC\",\"x\":\"{}\",\"y\":\"{}\"}}",
        key.x, key.y
    );
    URL_SAFE_NO_PAD.encode(Sha256::digest(canonical.as_bytes()))
}
/// Bootstrap challenges use compact JWS; the received payload bytes are verified as-is.
pub fn sign_jws(key: &SigningKey, purpose: &str, payload: &impl Serialize) -> Result<String> {
    let header = serde_json::json!({"alg":"ES256","typ":purpose,"kid":key_id(key.verifying_key())});
    let data = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header)?),
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(payload)?)
    );
    let signature: Signature = key.sign(data.as_bytes());
    Ok(format!(
        "{data}.{}",
        URL_SAFE_NO_PAD.encode(signature.to_bytes())
    ))
}
pub fn verify_jws<T: DeserializeOwned>(
    key: &VerifyingKey,
    purpose: &str,
    value: &str,
) -> Result<T> {
    anyhow::ensure!(value.len() <= 16384, "signed message too large");
    let parts: Vec<_> = value.split('.').collect();
    anyhow::ensure!(parts.len() == 3, "invalid JWS");
    let header: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0])?)?;
    let expected = serde_json::json!({"alg":"ES256","typ":purpose,"kid":key_id(key)});
    anyhow::ensure!(header == expected, "unexpected JWS header or purpose");
    let sig = Signature::from_slice(&URL_SAFE_NO_PAD.decode(parts[2])?)?;
    key.verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &sig)?;
    Ok(serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1])?)?)
}
pub fn jws_payload_json(value: &str) -> Result<serde_json::Value> {
    anyhow::ensure!(value.len() <= 16384, "signed message too large");
    let payload = value.split('.').nth(1).context("invalid JWS")?;
    Ok(serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jws_cannot_cross_purpose_and_private_key_roundtrips() {
        let key = new_key();
        let value = serde_json::json!({"probeId":random_id()});
        let proof = sign_jws(&key, "meow-host-challenge+jwt", &value).unwrap();
        let decoded: serde_json::Value =
            verify_jws(key.verifying_key(), "meow-host-challenge+jwt", &proof).unwrap();
        assert_eq!(decoded, value);
        assert!(verify_jws::<serde_json::Value>(
            key.verifying_key(),
            "meow-host-claim-receipt+jwt",
            &proof
        )
        .is_err());
        assert_eq!(
            key_id(
                decode_key(&encode_key(&key).unwrap())
                    .unwrap()
                    .verifying_key()
            ),
            key_id(key.verifying_key())
        );
        assert_eq!(
            key_id(&verifying_key(&public_key(key.verifying_key())).unwrap()),
            key_id(key.verifying_key())
        );
    }
}
