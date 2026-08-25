use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::V1;
pub const PLUGIN_TYPE: &str = "llm";
pub const RUNTIME_STATUS: &str = "runtime/status";
pub const RUNTIME_RETRY: &str = "runtime/retry";
pub const MODEL_LIST: &str = "model/list";
pub const MODEL_DOWNLOAD: &str = "model/download";
pub const MODEL_DOWNLOAD_STATUS: &str = "model/download/status";
pub const MODEL_DOWNLOAD_CANCEL: &str = "model/download/cancel";
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
