use serde::{Deserialize, Serialize};

/// Node operations use the Full App result contract. Presentation stays at the App boundary.
pub use conversation_api::execution::{
    FacadeProblem as OperationProblem, FacadeResponse as OperationResult,
};

pub mod contracts;
pub mod settings;
pub mod status;

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
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RuntimeResponse {
    pub version: String,
    pub result: OperationResult,
}

/// Reject presentation-only error data at the Node boundary. A successful string
/// is legitimate business data and must never be interpreted as Essential YAML.
pub fn validate_operation_result(result: &OperationResult) -> Result<(), &'static str> {
    if matches!(result, OperationResult::Error { data: Some(_), .. }) {
        return Err("Node operations require Full error details, not Essential data");
    }
    Ok(())
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
    fn runtime_response_requires_explicit_full_results() {
        use serde_json::json;
        for result in [
            json!({"status":"success","data":[]}),
            json!({"status":"success"}),
            json!({"status":"partial","data":[],"warnings":[{"code":"INCOMPLETE","message":"retry"}]}),
            json!({"status":"error","error":{"code":"PLUGIN_BUSY","message":"retry","details":{"after":2}}}),
            json!({"status":"cancelled"}),
        ] {
            let value = json!({"version":"v1","result":result});
            let response: RuntimeResponse = serde_json::from_value(value.clone()).unwrap();
            validate_operation_result(&response.result).unwrap();
            assert_eq!(serde_json::to_value(response).unwrap(), value);
        }
        assert!(
            serde_json::from_value::<RuntimeResponse>(json!({"version":"v1","data":[]})).is_err()
        );
        let result: OperationResult = serde_json::from_value(
            json!({"status":"error","error":{"code":"X","message":"x"},"data":"yaml"}),
        )
        .unwrap();
        assert!(validate_operation_result(&result).is_err());
    }

    #[test]
    fn manifest_matches_internal_targets() {
        let manifest: serde_json::Value = serde_json::from_str(PROTOCOL_V1_MANIFEST).unwrap();
        assert_eq!(manifest["version"], V1);
        assert_eq!(manifest["status"]["contract"], STATUS_CONTRACT);
        assert_eq!(manifest["status"]["queryTarget"], status::STATUS_TARGET);
        assert_eq!(
            manifest["status"]["changedTarget"],
            status::STATUS_CHANGED_TARGET
        );
        assert_eq!(
            manifest["settings"]["changedTarget"],
            settings::CHANGED_TARGET
        );
        assert_eq!(
            manifest["llm"]["completeTarget"],
            llm_api::service::COMPLETE_TARGET
        );
        assert_eq!(
            manifest["llm"]["ttsSynthesizeTarget"],
            contracts::llm::TTS_SYNTHESIZE_TARGET
        );
    }
}
