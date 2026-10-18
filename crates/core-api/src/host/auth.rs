//! Host public-key types and the cloud signing-key directory.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const KEY_REFRESH_INTERVAL_SECONDS: u64 = 24 * 60 * 60;
pub const CLAIM_TOKEN_REQUIRED: &str = "HOST_CLAIM_TOKEN_REQUIRED";

/// Public P-256 point; x/y are exactly 32 bytes, base64url without padding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PublicKey {
    pub x: String,
    pub y: String,
}

/// Verification-only permits a signing-key overlap; disabled keys accept no proofs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SigningKeyStatus {
    Active,
    VerifyOnly,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigningKey {
    pub kid: String,
    pub status: SigningKeyStatus,
    /// RS256 JWK modulus and exponent, base64url without padding.
    pub n: String,
    pub e: String,
}

/// A complete authoritative snapshot from the configured HTTPS cloud.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SigningKeySet {
    pub version: u64,
    pub keys: Vec<SigningKey>,
}

impl SigningKeySet {
    pub fn validate_update(&self, previous: Option<&Self>) -> Result<(), &'static str> {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        if self.version == 0 || self.version > 9_007_199_254_740_991 {
            return Err("invalid key set version");
        }
        if self.keys.is_empty() || self.keys.len() > 128 {
            return Err("invalid key set size");
        }
        let mut kids = BTreeSet::new();
        let mut active = 0;
        for key in &self.keys {
            if key.kid.is_empty() || key.kid.len() > 128 || !kids.insert(&key.kid) {
                return Err("invalid or duplicate signing key ID");
            }
            let n = URL_SAFE_NO_PAD
                .decode(&key.n)
                .map_err(|_| "invalid RSA modulus")?;
            let e = URL_SAFE_NO_PAD
                .decode(&key.e)
                .map_err(|_| "invalid RSA exponent")?;
            if !(256..=512).contains(&n.len())
                || n.first().is_some_and(|v| *v < 128)
                || e != [1, 0, 1]
            {
                return Err("unsupported RSA public key");
            }
            active += usize::from(key.status == SigningKeyStatus::Active);
        }
        if active != 1 {
            return Err("exactly one active signing key is required");
        }
        if let Some(old) = previous {
            if self.version < old.version {
                return Err("key set rollback");
            }
            let index: BTreeMap<_, _> = old.keys.iter().map(|k| (&k.kid, k)).collect();
            for key in &self.keys {
                if let Some(prior) = index.get(&key.kid) {
                    if key.n != prior.n || key.e != prior.e {
                        return Err("signing key ID was reused");
                    }
                    if prior.status == SigningKeyStatus::Disabled
                        && key.status != SigningKeyStatus::Disabled
                    {
                        return Err("disabled signing key was reactivated");
                    }
                }
            }
            let next: BTreeMap<_, _> = self.keys.iter().map(|k| (&k.kid, k)).collect();
            if self.version == old.version && next != index {
                return Err("conflicting key set version");
            }
            // Keep tombstones: an offline Host must learn emergency disablement too.
            if old
                .keys
                .iter()
                .any(|k| k.status == SigningKeyStatus::Disabled && !next.contains_key(&k.kid))
            {
                return Err("disabled signing key tombstone was removed");
            }
        }
        Ok(())
    }

    pub fn permits_token(&self, kid: &str) -> bool {
        self.keys
            .iter()
            .any(|key| key.kid == kid && key.status != SigningKeyStatus::Disabled)
    }
}
