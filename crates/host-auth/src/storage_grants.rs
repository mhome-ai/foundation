//! Reusable read capabilities without one retained record per transfer.
use anyhow::{ensure, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::Sha256;
use std::collections::HashMap;
use tokio::sync::Mutex;

type HmacSha256 = Hmac<Sha256>;

struct ScopeKey {
    secret: [u8; 32],
    expires: u64,
}

#[derive(Serialize, Deserialize)]
struct Claims<T> {
    scope: String,
    expires: u64,
    value: T,
}

/// Keys are scoped to a binding; expired keys are pruned when issuing capabilities.
/// Revocation removes that key; issuing again cannot revive earlier capabilities.
#[derive(Default)]
pub struct ReadGrants {
    scopes: Mutex<HashMap<String, ScopeKey>>,
}

impl ReadGrants {
    pub async fn issue<T: Serialize>(&self, scope: &str, value: T, expires: u64) -> Result<String> {
        let now = now_ms();
        ensure!(expires > now, "Read capability already expired");
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&Claims {
            scope: scope.to_owned(),
            expires,
            value,
        })?);
        // Bound the token on issuance as well as on untrusted input.
        ensure!(payload.len() + 44 <= 16384, "Read capability too large");
        let mut scopes = self.scopes.lock().await;
        scopes.retain(|_, key| key.expires > now);
        let key = scopes.entry(scope.to_owned()).or_insert_with(|| {
            let mut secret = [0; 32];
            OsRng.fill_bytes(&mut secret);
            ScopeKey { secret, expires }
        });
        key.expires = key.expires.max(expires);
        let mut mac = HmacSha256::new_from_slice(&key.secret)?;
        mac.update(payload.as_bytes());
        Ok(format!(
            "{payload}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        ))
    }

    pub async fn verify<T: DeserializeOwned>(&self, token: &str) -> Result<T> {
        self.verify_at(token, now_ms()).await
    }

    async fn verify_at<T: DeserializeOwned>(&self, token: &str, now: u64) -> Result<T> {
        ensure!(token.len() <= 16384, "Read capability too large");
        let (payload, signature) = token
            .split_once('.')
            .ok_or_else(|| anyhow::anyhow!("Invalid read capability"))?;
        let claims: Claims<T> = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
        let scopes = self.scopes.lock().await;
        let key = scopes
            .get(&claims.scope)
            .ok_or_else(|| anyhow::anyhow!("Read capability revoked"))?;
        let mut mac = HmacSha256::new_from_slice(&key.secret)?;
        mac.update(payload.as_bytes());
        mac.verify_slice(&URL_SAFE_NO_PAD.decode(signature)?)?;
        ensure!(claims.expires > now, "Read capability expired");
        Ok(claims.value)
    }

    pub async fn revoke(&self, scope: &str) {
        self.scopes.lock().await.remove(scope);
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    #[tokio::test]
    async fn repeated_reads_keep_only_one_scope_key_and_retain_independent_expiry() {
        let grants = ReadGrants::default();
        let expiry = now_ms() + 900000;
        let first = grants
            .issue("space", json!({"object":"first"}), expiry)
            .await
            .unwrap();
        for n in 0..5000 {
            let token = grants
                .issue("space", json!({"object":n}), expiry + 1)
                .await
                .unwrap();
            assert_eq!(
                grants.verify::<Value>(&token).await.unwrap(),
                json!({"object":n})
            );
        }
        assert_eq!(grants.scopes.lock().await.len(), 1);
        assert!(grants.verify_at::<Value>(&first, expiry).await.is_err());
        assert_eq!(
            grants.verify::<Value>(&first).await.unwrap(),
            json!({"object":"first"})
        );
    }

    #[tokio::test]
    async fn tampering_other_authorities_and_expired_keys_cannot_authorize_reads() {
        let grants = ReadGrants::default();
        let expiry = now_ms() + 900000;
        let token = grants.issue("space", "object", expiry).await.unwrap();
        let (payload, signature) = token.split_once('.').unwrap();
        let mut claims: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).unwrap()).unwrap();
        claims["value"] = json!("another-object");
        let forged = format!(
            "{}.{signature}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&claims).unwrap())
        );
        assert!(grants.verify::<String>(&forged).await.is_err());
        assert!(ReadGrants::default()
            .verify::<String>(&token)
            .await
            .is_err());
        assert!(grants.issue("space", "object", now_ms()).await.is_err());
        assert!(grants.verify::<String>(&"x".repeat(16385)).await.is_err());
        grants.scopes.lock().await.get_mut("space").unwrap().expires = 0;
        grants
            .issue("another-space", "object", expiry)
            .await
            .unwrap();
        assert!(grants.verify::<String>(&token).await.is_err());
    }

    #[tokio::test]
    async fn revocation_is_scope_specific_and_reissue_never_revives_old_tokens() {
        let grants = ReadGrants::default();
        let expiry = now_ms() + 900000;
        let old = grants.issue("space-a", "object", expiry).await.unwrap();
        let other = grants.issue("space-b", "object", expiry).await.unwrap();
        grants.revoke("space-a").await;
        assert!(grants.verify::<String>(&old).await.is_err());
        assert!(grants.verify::<String>(&other).await.is_ok());
        let new = grants.issue("space-a", "object", expiry).await.unwrap();
        assert!(grants.verify::<String>(&new).await.is_ok());
        assert!(grants.verify::<String>(&old).await.is_err());
    }
}
