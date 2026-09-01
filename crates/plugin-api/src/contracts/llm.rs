use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::V1;
pub const NODE_TYPE: &str = "llm";
pub const RUNTIME_TARGET_PREFIX: &str = "/llm/app/";
pub const RUNTIME_STATUS: &str = "runtime/status";
pub const RUNTIME_RETRY: &str = "runtime/retry";
pub const MODEL_LIST: &str = "model/list";
pub const MODEL_DOWNLOAD: &str = "model/download";
pub const MODEL_DOWNLOAD_STATUS: &str = "model/download/status";
pub const MODEL_DOWNLOAD_CANCEL: &str = "model/download/cancel";
pub const MODEL_IMPORT_START: &str = "model/import/start";
pub const MODEL_IMPORT_STATUS: &str = "model/import/status";
pub const MODEL_IMPORT_FINALIZE: &str = "model/import/finalize";
pub const MODEL_IMPORT_CANCEL: &str = "model/import/cancel";
pub const MODEL_IMPORT_UPLOAD_PATH_PREFIX: &str = "/llm/model/import/upload";
pub const MODEL_IMPORT_OFFSET_HEADER: &str = "x-upload-offset";
pub const MODEL_IMPORT_CHUNK_SHA256_HEADER: &str = "x-chunk-sha256";
pub const MODEL_DELETE: &str = "model/delete";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelDownloadRequest {
    pub model_id: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelRequest {
    pub model_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelImportStartRequest {
    pub display_name: String,
    pub file_name: String,
    pub artifact_sha256: String,
    pub size_bytes: u64,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelImportOperationRequest {
    pub operation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ModelImportState {
    Uploading,
    Importing,
    Verifying,
    Available,
    Cancelling,
    Cancelled,
    Error,
}

impl ModelImportState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Available | Self::Cancelled | Self::Error)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportError {
    pub code: String,
    pub detail: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportSnapshot {
    pub operation_id: String,
    pub model_id: String,
    pub display_name: String,
    pub artifact_sha256: String,
    pub size_bytes: u64,
    pub received_bytes: u64,
    pub state: ModelImportState,
    pub status: String,
    pub started_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
    pub cancellable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ModelImportError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportUploadGrant {
    pub bearer_token: String,
    pub chunk_size_bytes: u64,
    pub expires_at_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportStartResponse {
    pub ok: bool,
    pub started: bool,
    pub operation: ModelImportSnapshot,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upload: Option<ModelImportUploadGrant>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportStatusResponse {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<ModelImportSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelImportChunkResponse {
    pub ok: bool,
    pub operation_id: String,
    pub received_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn import_start_request_is_strict_and_uses_public_names() {
        let value = serde_json::to_value(ModelImportStartRequest {
            display_name: "Local model".to_string(),
            file_name: "model.gguf".to_string(),
            artifact_sha256: "a".repeat(64),
            size_bytes: 1024,
            idempotency_key: "import-1".to_string(),
        })
        .expect("serialize import request");
        assert_eq!(value["artifactSha256"], "a".repeat(64));
        assert!(value.get("sourcePath").is_none());
        assert!(value.get("scopeId").is_none());

        let mut invalid = value;
        invalid["sourcePath"] = serde_json::json!("/private/model.gguf");
        assert!(serde_json::from_value::<ModelImportStartRequest>(invalid).is_err());
    }

    #[test]
    fn import_terminal_states_are_explicit() {
        assert!(ModelImportState::Available.is_terminal());
        assert!(ModelImportState::Cancelled.is_terminal());
        assert!(ModelImportState::Error.is_terminal());
        assert!(!ModelImportState::Uploading.is_terminal());
    }
}
