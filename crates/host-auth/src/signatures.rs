use anyhow::{Context, Result};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use core_api::host::auth::{PublicKey, PROTOCOL};
use http::{HeaderMap, HeaderValue};
use p256::{
    ecdsa::{
        signature::{Signer, Verifier},
        Signature, SigningKey, VerifyingKey,
    },
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
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
pub fn digest(body: &[u8]) -> String {
    format!("sha-256=:{}:", STANDARD.encode(Sha256::digest(body)))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestIdentity {
    pub user_id: String,
    pub host_id: String,
    pub client_key_id: String,
    pub boot_id: String,
    pub request_id: String,
    pub created: u64,
    pub expires: u64,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResponseIdentity {
    pub host_id: String,
    pub request_digest: String,
}
fn single<'a>(headers: &'a HeaderMap, name: &str) -> Result<&'a str> {
    anyhow::ensure!(
        headers.get_all(name).iter().count() == 1,
        "missing or duplicate signature header"
    );
    let value = headers.get(name).unwrap().to_str()?;
    anyhow::ensure!(value.len() <= 8192, "signature header too large");
    Ok(value)
}
fn put(headers: &mut HeaderMap, name: &'static str, value: String) -> Result<()> {
    headers.insert(name, HeaderValue::from_str(&value)?);
    Ok(())
}
fn input(components: &str, kid: &str) -> String {
    format!("({components});keyid=\"{kid}\";alg=\"ecdsa-p256-sha256\";tag=\"{PROTOCOL}\"")
}
const REQUEST_COMPONENTS: &str =
    "\"@method\" \"@target-uri\" \"content-digest\" \"content-type\" \"x-meow-auth\"";
const RESPONSE_COMPONENTS: &str = "\"@status\" \"content-digest\" \"content-type\" \"x-meow-auth\"";
fn sign(headers: &mut HeaderMap, key: &SigningKey, base: &str, params: String) -> Result<()> {
    let signature: Signature = key.sign(base.as_bytes());
    put(headers, "signature-input", format!("meow={params}"))?;
    put(
        headers,
        "signature",
        format!("meow=:{}:", STANDARD.encode(signature.to_bytes())),
    )
}
fn verify(headers: &HeaderMap, key: &VerifyingKey, base: &str, params: &str) -> Result<()> {
    anyhow::ensure!(
        single(headers, "signature-input")? == format!("meow={params}"),
        "unsupported signature profile"
    );
    let encoded = single(headers, "signature")?
        .strip_prefix("meow=:")
        .and_then(|v| v.strip_suffix(':'))
        .context("invalid signature field")?;
    let sig = Signature::from_slice(&STANDARD.decode(encoded)?)?;
    key.verify(base.as_bytes(), &sig)
        .context("invalid Host message signature")
}
fn request_base(headers: &HeaderMap, method: &str, uri: &str, params: &str) -> Result<String> {
    anyhow::ensure!(
        !method.contains(['\r', '\n']) && !uri.contains(['\r', '\n']),
        "invalid target"
    );
    Ok(format!("\"@method\": {method}\n\"@target-uri\": {uri}\n\"content-digest\": {}\n\"content-type\": {}\n\"x-meow-auth\": {}\n\"@signature-params\": {params}",single(headers,"content-digest")?,single(headers,"content-type")?,single(headers,"x-meow-auth")?))
}
fn response_base(headers: &HeaderMap, status: u16, params: &str) -> Result<String> {
    Ok(format!("\"@status\": {status}\n\"content-digest\": {}\n\"content-type\": {}\n\"x-meow-auth\": {}\n\"@signature-params\": {params}",single(headers,"content-digest")?,single(headers,"content-type")?,single(headers,"x-meow-auth")?))
}
fn prepare(headers: &mut HeaderMap, body: &[u8], identity: &impl Serialize) -> Result<()> {
    put(headers, "content-digest", digest(body))?;
    put(headers, "content-type", "application/json".into())?;
    put(
        headers,
        "x-meow-auth",
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(identity)?),
    )
}
pub fn sign_request(
    key: &SigningKey,
    method: &str,
    uri: &str,
    body: &[u8],
    identity: &RequestIdentity,
) -> Result<HeaderMap> {
    anyhow::ensure!(
        identity.client_key_id == key_id(key.verifying_key()),
        "Client signing identity mismatch"
    );
    let mut headers = HeaderMap::new();
    prepare(&mut headers, body, identity)?;
    let params = input(REQUEST_COMPONENTS, &identity.client_key_id);
    let base = request_base(&headers, method, uri, &params)?;
    sign(&mut headers, key, &base, params)?;
    Ok(headers)
}
/// Parsing is not authentication; only use this to locate a previously enrolled public key.
pub fn request_identity(headers: &HeaderMap) -> Result<RequestIdentity> {
    Ok(serde_json::from_slice(
        &URL_SAFE_NO_PAD.decode(single(headers, "x-meow-auth")?)?,
    )?)
}
pub fn verify_request(
    headers: &HeaderMap,
    key: &VerifyingKey,
    method: &str,
    uri: &str,
    body: &[u8],
) -> Result<RequestIdentity> {
    let identity = request_identity(headers)?;
    anyhow::ensure!(
        identity.client_key_id == key_id(key),
        "Client key ID mismatch"
    );
    anyhow::ensure!(
        single(headers, "content-digest")? == digest(body),
        "request body digest mismatch"
    );
    let params = input(REQUEST_COMPONENTS, &identity.client_key_id);
    verify(
        headers,
        key,
        &request_base(headers, method, uri, &params)?,
        &params,
    )?;
    Ok(identity)
}
pub fn request_digest(headers: &HeaderMap, method: &str, uri: &str) -> Result<String> {
    let identity = request_identity(headers)?;
    let params = input(REQUEST_COMPONENTS, &identity.client_key_id);
    let base = request_base(headers, method, uri, &params)?;
    Ok(URL_SAFE_NO_PAD.encode(Sha256::digest(
        format!("{base}\n{}", single(headers, "signature")?).as_bytes(),
    )))
}
pub fn sign_response(
    key: &SigningKey,
    status: u16,
    body: &[u8],
    identity: &ResponseIdentity,
) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    prepare(&mut headers, body, identity)?;
    let params = input(RESPONSE_COMPONENTS, &key_id(key.verifying_key()));
    let base = response_base(&headers, status, &params)?;
    sign(&mut headers, key, &base, params)?;
    Ok(headers)
}
pub fn verify_response(
    headers: &HeaderMap,
    key: &VerifyingKey,
    status: u16,
    body: &[u8],
    expected: &ResponseIdentity,
) -> Result<()> {
    let identity: ResponseIdentity =
        serde_json::from_slice(&URL_SAFE_NO_PAD.decode(single(headers, "x-meow-auth")?)?)?;
    anyhow::ensure!(
        &identity == expected,
        "response belongs to another Host or request"
    );
    anyhow::ensure!(
        single(headers, "content-digest")? == digest(body),
        "response body digest mismatch"
    );
    let params = input(RESPONSE_COMPONENTS, &key_id(key));
    verify(
        headers,
        key,
        &response_base(headers, status, &params)?,
        &params,
    )
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
    fn identity(key: &SigningKey) -> RequestIdentity {
        RequestIdentity {
            user_id: "alice".into(),
            host_id: "host".into(),
            client_key_id: key_id(key.verifying_key()),
            boot_id: random_id(),
            request_id: random_id(),
            created: 100,
            expires: 120,
        }
    }
    #[test]
    fn request_signature_binds_target_body_and_identity() {
        let key = new_key();
        let id = identity(&key);
        let body = b"{\"component\":\"matter\"}";
        let url = "http://192.168.0.2:20280/v1/runtime?a=1";
        let headers = sign_request(&key, "POST", url, body, &id).unwrap();
        verify_request(&headers, key.verifying_key(), "POST", url, body).unwrap();
        assert!(verify_request(&headers, key.verifying_key(), "GET", url, body).is_err());
        assert!(verify_request(
            &headers,
            key.verifying_key(),
            "POST",
            "http://192.168.0.2:20280/v1/runtime?a=2",
            body
        )
        .is_err());
        assert!(verify_request(&headers, key.verifying_key(), "POST", url, b"{}").is_err());
        let mut changed = headers.clone();
        changed.append("x-meow-auth", HeaderValue::from_static("e30"));
        assert!(verify_request(&changed, key.verifying_key(), "POST", url, body).is_err());
        assert!(verify_request(&headers, new_key().verifying_key(), "POST", url, body).is_err());
    }
    #[test]
    fn unsigned_or_other_request_error_cannot_trigger_reauth() {
        let key = new_key();
        let id = identity(&key);
        let req = sign_request(&key, "POST", "http://host/v1/runtime", b"{}", &id).unwrap();
        let binding = ResponseIdentity {
            host_id: "host".into(),
            request_digest: request_digest(&req, "POST", "http://host/v1/runtime").unwrap(),
        };
        let body = b"{\"code\":\"HOST_REAUTH_REQUIRED\"}";
        let headers = sign_response(&key, 401, body, &binding).unwrap();
        verify_response(&headers, key.verifying_key(), 401, body, &binding).unwrap();
        assert!(verify_response(&headers, key.verifying_key(), 200, body, &binding).is_err());
        let other = ResponseIdentity {
            host_id: "host".into(),
            request_digest: random_id(),
        };
        assert!(verify_response(&headers, key.verifying_key(), 401, body, &other).is_err());
        assert!(
            verify_response(&HeaderMap::new(), key.verifying_key(), 401, body, &binding).is_err()
        );
    }
    #[test]
    fn jws_cannot_cross_purpose_and_private_key_roundtrips() {
        let key = new_key();
        let value = serde_json::json!({"probeId":random_id()});
        let proof = sign_jws(&key, "meow-host-context", &value).unwrap();
        let decoded: serde_json::Value =
            verify_jws(key.verifying_key(), "meow-host-context", &proof).unwrap();
        assert_eq!(decoded, value);
        assert!(verify_jws::<serde_json::Value>(
            key.verifying_key(),
            "meow-host-enrollment",
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
