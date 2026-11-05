//! Bounded temporary text resources. References are always resolved within the caller's Space.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PREPARE_TARGET: &str = "/app/resource/transfer/prepare";
pub const READ_ROUTE: &str = "/bridge/resource/transfer/read";
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
pub const TTL_MS: u64 = 300_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferReference {
    pub id: String,
    pub expires_at: u64,
    pub size_bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum TextPayload {
    Inline { text: String },
    Object { reference: TransferReference },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareTransferRequest {
    pub size_bytes: usize,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TransferGrant {
    pub reference: TransferReference,
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

impl TransferReference {
    pub fn validate(&self, now_ms: u64) -> Result<(), &'static str> {
        if self.id.len() != 32
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            || !(1..=MAX_BYTES).contains(&self.size_bytes)
            || self
                .sha256
                .parse::<crate::extension_sources::SnapshotId>()
                .is_err()
            || self.expires_at <= now_ms
            || self.expires_at.saturating_sub(now_ms) > TTL_MS + 60_000
        {
            return Err("Invalid or expired resource transfer reference");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn references_are_small_bounded_and_expire() {
        let mut reference = TransferReference {
            id: "a".repeat(32),
            expires_at: 301_000,
            size_bytes: MAX_BYTES,
            sha256: "b".repeat(64),
        };
        assert!(reference.validate(1000).is_ok());
        assert!(reference.validate(301_000).is_err());
        reference.size_bytes += 1;
        assert!(reference.validate(1000).is_err());
        reference.size_bytes = 1;
        reference.id = "../other".into();
        assert!(reference.validate(1000).is_err());
        assert!(serde_json::from_value::<TextPayload>(
            serde_json::json!({"kind":"object", "url":"http://arbitrary"})
        )
        .is_err());
    }
}
