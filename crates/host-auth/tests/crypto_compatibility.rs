//! Compare the released P-256 implementation with the current one using only
//! fictional keys. Old crypto dependencies are confined to this test oracle.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as B64, Engine};
use core_api::host::auth::PublicKey;
use elliptic_curve::Generate;
use host_auth::signatures::*;
use p256::ecdsa::SigningKey;
use p256_legacy::{
    ecdsa::{
        signature::{Signer, Verifier},
        Signature as OldSignature, SigningKey as OldKey,
    },
    pkcs8::{DecodePrivateKey, EncodePrivateKey},
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const PURPOSE: &str = "meow-host-challenge+jwt";

fn old_public(key: &OldKey) -> PublicKey {
    let point = key.verifying_key().to_encoded_point(false);
    PublicKey {
        x: B64.encode(point.x().unwrap()),
        y: B64.encode(point.y().unwrap()),
    }
}

fn old_id(key: &OldKey) -> String {
    let public = old_public(key);
    let canonical = format!(
        "{{\"crv\":\"P-256\",\"kty\":\"EC\",\"x\":\"{}\",\"y\":\"{}\"}}",
        public.x, public.y
    );
    B64.encode(Sha256::digest(canonical.as_bytes()))
}

fn old_jws(key: &OldKey, header: Value, payload: &[u8]) -> String {
    let data = format!(
        "{}.{}",
        B64.encode(serde_json::to_vec(&header).unwrap()),
        B64.encode(payload)
    );
    let signature: OldSignature = key.sign(data.as_bytes());
    format!("{data}.{}", B64.encode(signature.to_bytes()))
}

fn header(key: &OldKey) -> Value {
    json!({"alg":"ES256", "typ":PURPOSE, "kid":old_id(key)})
}

#[test]
fn persisted_pkcs8_public_coordinates_and_thumbprints_are_identical() {
    // Include a private scalar with leading zeros and public coordinates with
    // leading zeros: neither may be shortened when producing a JWK thumbprint.
    let mut keys = vec![[7u8; 32]];
    let mut x_zero = false;
    let mut y_zero = false;
    for scalar in 1u32..=4096 {
        let mut bytes = [0; 32];
        bytes[28..].copy_from_slice(&scalar.to_be_bytes());
        let key = OldKey::from_slice(&bytes).unwrap();
        let point = key.verifying_key().to_encoded_point(false);
        let x = point.x().unwrap()[0] == 0;
        let y = point.y().unwrap()[0] == 0;
        if scalar == 1 || (x && !x_zero) || (y && !y_zero) {
            keys.push(bytes);
        }
        x_zero |= x;
        y_zero |= y;
        if x_zero && y_zero {
            break;
        }
    }
    assert!(
        x_zero && y_zero,
        "fixture corpus must cover both zero-prefixed coordinates"
    );
    for bytes in keys {
        let old = OldKey::from_slice(&bytes).unwrap();
        let encoded = B64.encode(old.to_pkcs8_der().unwrap().as_bytes());
        let new = decode_key(&encoded).unwrap();
        assert_eq!(new.to_bytes().as_slice(), bytes);
        assert_eq!(encode_key(&new).unwrap(), encoded);
        let reread =
            OldKey::from_pkcs8_der(&B64.decode(encode_key(&new).unwrap()).unwrap()).unwrap();
        assert_eq!(&reread.to_bytes()[..], bytes);
        assert_eq!(public_key(new.verifying_key()), old_public(&old));
        assert_eq!(key_id(new.verifying_key()), old_id(&old));
        assert_eq!(
            verifying_key(&old_public(&old)).unwrap(),
            *new.verifying_key()
        );
    }
}

#[test]
fn old_and_new_jws_are_mutually_verifiable_and_byte_identical() {
    let old = OldKey::from_slice(&[7; 32]).unwrap();
    let new = SigningKey::from_slice(&[7; 32]).unwrap();
    let payload = json!({"probeId":"fictional", "iat":100, "exp":400, "name":"测试"});
    let old_token = old_jws(&old, header(&old), &serde_json::to_vec(&payload).unwrap());
    assert_eq!(
        verify_jws::<Value>(new.verifying_key(), PURPOSE, &old_token).unwrap(),
        payload
    );
    let new_token = sign_jws(&new, PURPOSE, &payload).unwrap();
    assert_eq!(new_token, old_token);
    let (data, encoded_signature) = new_token.rsplit_once('.').unwrap();
    let signature = B64.decode(encoded_signature).unwrap();
    assert_eq!(signature.len(), 64); // ES256 uses R||S, never ASN.1 DER.
    old.verifying_key()
        .verify(
            data.as_bytes(),
            &OldSignature::from_slice(&signature).unwrap(),
        )
        .unwrap();

    // Verification must use the received bytes, not parse/re-serialize JSON.
    let spaced = b"{ \"exp\":400, \"iat\":100, \"probeId\":\"fictional\" }";
    let token = old_jws(&old, header(&old), spaced);
    assert_eq!(
        verify_jws::<Value>(new.verifying_key(), PURPOSE, &token).unwrap(),
        serde_json::from_slice::<Value>(spaced).unwrap()
    );
}

#[test]
fn both_s_representations_remain_accepted() {
    let old = OldKey::from_slice(&[7; 32]).unwrap();
    let new = SigningKey::from_slice(&[7; 32]).unwrap();
    let token = old_jws(&old, header(&old), b"{}");
    let (data, encoded) = token.rsplit_once('.').unwrap();
    let signature = OldSignature::from_slice(&B64.decode(encoded).unwrap()).unwrap();
    let opposite =
        OldSignature::from_scalars(signature.r().to_bytes(), (-*signature.s()).to_bytes()).unwrap();
    assert_ne!(signature.to_bytes(), opposite.to_bytes());
    for signature in [signature, opposite] {
        old.verifying_key()
            .verify(data.as_bytes(), &signature)
            .unwrap();
        let token = format!("{data}.{}", B64.encode(signature.to_bytes()));
        assert_eq!(
            verify_jws::<Value>(new.verifying_key(), PURPOSE, &token).unwrap(),
            json!({})
        );
    }
}

#[test]
fn invalid_headers_payloads_keys_and_signature_encodings_stay_rejected() {
    let old = OldKey::from_slice(&[7; 32]).unwrap();
    let new = SigningKey::from_slice(&[7; 32]).unwrap();
    for field in ["alg", "typ", "kid"] {
        let mut bad = header(&old);
        bad[field] = json!("wrong");
        let token = old_jws(&old, bad, b"{}"); // Cryptographically valid, semantically wrong.
        assert!(verify_jws::<Value>(new.verifying_key(), PURPOSE, &token).is_err());
    }
    let mut extra = header(&old);
    extra["extra"] = json!(true);
    assert!(
        verify_jws::<Value>(new.verifying_key(), PURPOSE, &old_jws(&old, extra, b"{}")).is_err()
    );
    let token = old_jws(&old, header(&old), b"{}");
    let parts: Vec<_> = token.split('.').collect();
    let tampered = format!(
        "{}.{}.{}",
        parts[0],
        B64.encode(b"{\"admin\":true}"),
        parts[2]
    );
    assert!(verify_jws::<Value>(new.verifying_key(), PURPOSE, &tampered).is_err());
    let signature = OldSignature::from_slice(&B64.decode(parts[2]).unwrap()).unwrap();
    for bytes in [
        vec![0; 64],
        vec![0; 63],
        vec![0; 65],
        signature.to_der().as_bytes().to_vec(),
    ] {
        let bad = format!("{}.{}.{}", parts[0], parts[1], B64.encode(bytes));
        assert!(verify_jws::<Value>(new.verifying_key(), PURPOSE, &bad).is_err());
    }
    for bad in [
        "".to_owned(),
        "a.b".into(),
        format!("{token}.extra"),
        "x".repeat(16385),
    ] {
        assert!(verify_jws::<Value>(new.verifying_key(), PURPOSE, &bad).is_err());
    }
    assert!(verify_jws::<Value>(new_key().verifying_key(), PURPOSE, &token).is_err());
    for bytes in [vec![], vec![0; 32], vec![0xff; 32]] {
        assert!(decode_key(&B64.encode(bytes)).is_err());
    }
    for key in [
        PublicKey {
            x: B64.encode([0; 31]),
            y: B64.encode([0; 32]),
        },
        PublicKey {
            x: B64.encode([0; 32]),
            y: B64.encode([0; 32]),
        },
    ] {
        assert!(verifying_key(&key).is_err());
    }
}

#[test]
fn generation_requires_entropy_and_os_generated_keys_roundtrip() {
    use elliptic_curve::rand_core::{TryCryptoRng, TryRng, UnwrapErr};
    struct FailedRng;
    impl TryRng for FailedRng {
        type Error = std::io::Error;
        fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
            Err(std::io::Error::other("fixture entropy failure"))
        }
        fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
            Err(std::io::Error::other("fixture entropy failure"))
        }
        fn try_fill_bytes(&mut self, _: &mut [u8]) -> Result<(), Self::Error> {
            Err(std::io::Error::other("fixture entropy failure"))
        }
    }
    impl TryCryptoRng for FailedRng {}
    assert!(SigningKey::try_generate_from_rng(&mut FailedRng).is_err());
    // Generate::generate uses this same fail-closed adapter around the OS RNG.
    assert!(
        std::panic::catch_unwind(|| SigningKey::generate_from_rng(&mut UnwrapErr(FailedRng)))
            .is_err()
    );
    let key = new_key();
    assert_eq!(decode_key(&encode_key(&key).unwrap()).unwrap(), key);
}

#[cfg(feature = "tls")]
#[test]
fn old_persisted_identity_produces_the_same_tls_pin() {
    use rustls::pki_types::{pem::PemObject, CertificateDer};
    let old = OldKey::from_slice(&[7; 32]).unwrap();
    let key = decode_key(&B64.encode(old.to_pkcs8_der().unwrap().as_bytes())).unwrap();
    let (cert, pem) = host_auth::tls::certificate(&key).unwrap();
    assert_eq!(OldKey::from_pkcs8_pem(&pem).unwrap(), old);
    let der = CertificateDer::from_pem_slice(cert.as_bytes()).unwrap();
    assert_eq!(
        host_auth::tls::certificate_public_key(der.as_ref()).unwrap(),
        old_public(&old)
    );
    host_auth::tls::server_config(cert.as_bytes(), pem.as_bytes()).unwrap();
}
