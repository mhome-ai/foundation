//! Member-facing aliases of the transport-neutral Person domain contract.
pub use person_api::management::*;
pub const PREFIX: &str = "/app/person/";

/// Read-only image references available to members and their delegated agents.
pub const SUMMARY_IMAGES_BATCH_GET_TARGET: &str = "/app/person/summary-images/batch-get";
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SummaryImagesInput {
    /// Canonical `h:<uuid>` Person references. No count limit or pagination.
    pub ids: Vec<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryImagesOutput {
    /// Deduplicated in first-request order. Every identity has one Person URI.
    pub items: Vec<SummaryImage>,
    pub missing_ids: Vec<String>,
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SummaryImage {
    pub id: String,
    /// Scoped immutable Artifact URI; the caller retrieves its bytes through Artifact.
    pub uri: String,
}
