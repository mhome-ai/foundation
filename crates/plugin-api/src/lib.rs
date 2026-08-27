use serde::{Deserialize, Serialize};

pub mod contracts;
pub mod settings;

pub const V1: &str = "v1";
pub const CONTROL_TARGET_PREFIX: &str = "/app/plugin/control";
pub const CONTROL_TARGET_PREFIX_WITH_SLASH: &str = "/app/plugin/control/";
/// Canonical, language-neutral v1 wire manifest. Non-Rust consumers vendor
/// this immutable release artifact and validate their adapters at build time.
pub const NODE_PROTOCOL_V1_MANIFEST: &str = include_str!("../contract/node-protocol-v1.json");

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
    fn canonical_manifest_matches_rust_contract() {
        let manifest: serde_json::Value =
            serde_json::from_str(NODE_PROTOCOL_V1_MANIFEST).expect("valid protocol manifest");
        assert_eq!(manifest["pluginApi"]["version"], V1);
        assert_eq!(
            manifest["pluginApi"]["controlTargetPrefix"],
            CONTROL_TARGET_PREFIX
        );
        assert_eq!(
            manifest["pluginApi"]["settingsChangedTarget"],
            settings::CHANGED_TARGET
        );

        let request = serde_json::to_value(PluginControlRequest {
            plugin_id: "camera-1".to_string(),
            version: V1.to_string(),
            payload: serde_json::json!({}),
        })
        .expect("serialize request");
        assert_manifest_fields(&request, &manifest["pluginApi"]["controlRequestFields"]);

        let response = serde_json::to_value(PluginControlResponse {
            plugin_id: "camera-1".to_string(),
            plugin_type: "camera".to_string(),
            version: V1.to_string(),
            data: serde_json::json!({}),
        })
        .expect("serialize response");
        assert_manifest_fields(&response, &manifest["pluginApi"]["controlResponseFields"]);

        let changed = serde_json::to_value(settings::ChangedEvent {
            version: V1.to_string(),
            plugin_id: "camera-1".to_string(),
            plugin_type: "camera".to_string(),
            event_seq: 1,
            section: "recognition".to_string(),
            revision: 1,
        })
        .expect("serialize settings event");
        assert_manifest_fields(
            &changed,
            &manifest["pluginApi"]["settingsChangedEventFields"],
        );
        assert_eq!(
            manifest["llm"]["runtimeTargetPrefix"],
            contracts::llm::RUNTIME_TARGET_PREFIX
        );
        assert_eq!(
            manifest["llm"]["routes"]["modelImportStart"],
            contracts::llm::MODEL_IMPORT_START
        );
        assert_eq!(
            manifest["llm"]["routes"]["modelImportStatus"],
            contracts::llm::MODEL_IMPORT_STATUS
        );
        assert_eq!(
            manifest["llm"]["routes"]["modelImportFinalize"],
            contracts::llm::MODEL_IMPORT_FINALIZE
        );
        assert_eq!(
            manifest["llm"]["routes"]["modelImportCancel"],
            contracts::llm::MODEL_IMPORT_CANCEL
        );
        assert_eq!(
            manifest["llm"]["modelImportUploadPathPrefix"],
            contracts::llm::MODEL_IMPORT_UPLOAD_PATH_PREFIX
        );
        assert_eq!(
            manifest["llm"]["modelImportUploadHeaders"]["offset"],
            contracts::llm::MODEL_IMPORT_OFFSET_HEADER
        );
        assert_eq!(
            manifest["llm"]["modelImportUploadHeaders"]["chunkSha256"],
            contracts::llm::MODEL_IMPORT_CHUNK_SHA256_HEADER
        );
        let import = serde_json::to_value(contracts::llm::ModelImportStartRequest {
            display_name: "Local model".to_string(),
            file_name: "model.gguf".to_string(),
            artifact_sha256: "a".repeat(64),
            size_bytes: 4,
            idempotency_key: "import-1".to_string(),
        })
        .expect("serialize LLM import request");
        assert_manifest_fields(&import, &manifest["llm"]["modelImportStartRequestFields"]);
        let operation = serde_json::to_value(contracts::llm::ModelImportOperationRequest {
            operation_id: "operation-1".to_string(),
        })
        .expect("serialize LLM import operation request");
        assert_manifest_fields(
            &operation,
            &manifest["llm"]["modelImportOperationRequestFields"],
        );
        let chunk = serde_json::to_value(contracts::llm::ModelImportChunkResponse {
            ok: false,
            operation_id: "operation-1".to_string(),
            received_bytes: 4,
            reason: Some("UPLOAD_OFFSET_MISMATCH".to_string()),
            detail: Some("offset mismatch".to_string()),
        })
        .expect("serialize LLM import chunk response");
        assert_manifest_fields(&chunk, &manifest["llm"]["modelImportChunkResponseFields"]);
    }

    fn assert_manifest_fields(value: &serde_json::Value, expected: &serde_json::Value) {
        let mut actual_fields = value
            .as_object()
            .expect("wire object")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut expected_fields = expected
            .as_array()
            .expect("manifest fields")
            .iter()
            .map(|field| field.as_str().expect("field string").to_string())
            .collect::<Vec<_>>();
        actual_fields.sort();
        expected_fields.sort();
        assert_eq!(actual_fields, expected_fields);
    }

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
