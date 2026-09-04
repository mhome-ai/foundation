use serde::{Deserialize, Serialize};

pub mod contracts;
pub mod settings;

pub const V1: &str = "v1";
pub const STATUS_CONTRACT: &str = "mhome.node.status.v1";
pub const PROTOCOL_V1_MANIFEST: &str = include_str!("../../contract/node-runtime-protocol-v1.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeRequest<T> {
    pub version: String,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeResponse<T> {
    pub version: String,
    pub data: T,
}

pub fn require_version(actual: &str, expected: &str) -> Result<(), UnsupportedVersion> {
    if actual == expected {
        Ok(())
    } else {
        Err(UnsupportedVersion {
            actual: actual.to_string(),
            expected: expected.to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedVersion {
    actual: String,
    expected: String,
}

impl std::fmt::Display for UnsupportedVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "UNSUPPORTED_NODE_PROTOCOL_VERSION: requested version {}, supported version {}",
            self.actual, self.expected
        )
    }
}

impl std::error::Error for UnsupportedVersion {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_matches_internal_targets() {
        let manifest: serde_json::Value = serde_json::from_str(PROTOCOL_V1_MANIFEST).unwrap();
        assert_eq!(manifest["version"], V1);
        assert_eq!(
            manifest["settings"]["changedTarget"],
            settings::CHANGED_TARGET
        );
        assert_eq!(
            manifest["llm"]["completeTarget"],
            crate::llm::COMPLETE_TARGET
        );
    }
}
