//! Person management is Space-owned and available to every authenticated member.
pub use crate::{ClusterDetail, FaceCluster, FaceSampleView, Person, PersonStatus};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub person_ids: Option<Vec<String>>,
    pub unnamed: Option<bool>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonCreate {
    pub expected_epoch: String,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub notes: String,
    pub idempotency_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PersonUpdate {
    pub expected_epoch: String,
    pub id: String,
    pub expected_revision: String,
    pub name: String,
    pub tags: Vec<String>,
    pub notes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IdInput {
    pub id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionInput {
    pub expected_epoch: String,
    pub id: String,
    pub expected_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClusterAssign {
    pub expected_epoch: String,
    pub id: String,
    pub expected_revision: String,
    pub person_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClusterMerge {
    pub expected_epoch: String,
    pub source_id: String,
    pub source_revision: String,
    pub target_id: String,
    pub target_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClusterSplit {
    pub expected_epoch: String,
    pub id: String,
    pub expected_revision: String,
    pub sample_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleReview {
    pub expected_epoch: String,
    pub id: String,
    pub cluster_revision: String,
    pub accept: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResetInput {
    pub expected_epoch: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutationResult {
    pub epoch: String,
    pub gallery_revision: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageOutput {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportInput {
    pub password: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadInput {
    pub id: String,
    pub offset: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportStart {
    pub size_bytes: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportWrite {
    pub id: String,
    pub offset: usize,
    pub data_base64: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportCommit {
    pub id: String,
    pub password: String,
    pub expected_epoch: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferInfo {
    pub id: String,
    pub size_bytes: usize,
    pub chunk_bytes: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Chunk {
    pub data_base64: String,
    pub next_offset: usize,
    pub done: bool,
}
