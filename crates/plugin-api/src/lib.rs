use serde::{Deserialize, Serialize};

pub mod contracts;
pub mod settings;

pub const V1: &str = "v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginControlRequest<T> {
    pub plugin_id: String,
    pub version: String,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginControlResponse<T> {
    pub plugin_id: String,
    pub plugin_type: String,
    pub version: String,
    pub data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginRuntimeRequest<T> {
    pub version: String,
    pub payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginRuntimeResponse<T> {
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
    pub actual: String,
    pub expected: String,
}

impl std::fmt::Display for UnsupportedVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "UNSUPPORTED_PLUGIN_VERSION: requested version {}, supported version {}",
            self.actual, self.expected
        )
    }
}

impl std::error::Error for UnsupportedVersion {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_envelope_is_strict_and_uses_public_plugin_names() {
        let value = serde_json::to_value(PluginControlRequest {
            plugin_id: "camera-1".to_string(),
            version: contracts::camera::VERSION.to_string(),
            payload: contracts::camera::SettingsStatusRequest::default(),
        })
        .unwrap();
        assert_eq!(value["pluginId"], "camera-1");
        assert_eq!(value["version"], "v1");
        assert!(value.get("nodeId").is_none());

        let invalid = serde_json::from_value::<
            PluginControlRequest<contracts::camera::SettingsStatusRequest>,
        >(serde_json::json!({
            "pluginId": "camera-1",
            "version": "v1",
            "payload": {},
            "nodeId": "internal"
        }));
        assert!(invalid.is_err());
    }

    #[test]
    fn response_envelope_accepts_additive_fields() {
        let response =
            serde_json::from_value::<PluginControlResponse<serde_json::Value>>(serde_json::json!({
                "pluginId": "llm-1",
                "pluginType": "llm",
                "version": "v1",
                "data": {},
                "futureField": true
            }))
            .unwrap();
        assert_eq!(response.version, "v1");
    }
}
