use serde::{Deserialize, Serialize};
/// Immutable identity of comparable embeddings. Changing preprocessing requires a new id.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmbeddingSpace {
    pub id: String,
    pub dimensions: u32,
    pub recognizer_sha256: String,
    pub preprocessing: String,
    pub normalized: bool,
    pub metric: String,
}
impl EmbeddingSpace {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id.is_empty()
            || self.id.len() > 256
            || self.dimensions == 0
            || self.dimensions as usize > crate::MAX_VECTOR_DIMENSIONS
            || self.recognizer_sha256.len() != 64
            || !self
                .recognizer_sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit())
            || self.preprocessing.is_empty()
            || self.preprocessing.len() > 1024
            || !self.normalized
            || self.metric != "cosine"
        {
            return Err("unsupported embedding contract");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub notes: String,
    pub revision: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleState {
    Active,
    Pending,
    Rejected,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceCluster {
    pub id: String,
    pub embedding_space_id: String,
    pub person_id: Option<String>,
    pub stable: bool,
    pub revision: String,
    pub sample_count: u32,
    pub pending_count: u32,
    pub first_seen_at_ms: u64,
    pub last_seen_at_ms: u64,
    pub representative_sample_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FaceSampleView {
    pub id: String,
    pub cluster_id: String,
    pub state: SampleState,
    pub source_node_id: String,
    pub source_device_id: String,
    pub captured_at_ms: u64,
    pub quality: f32,
    pub has_image: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterDetail {
    pub cluster: FaceCluster,
    pub samples: Vec<FaceSampleView>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonStatus {
    pub epoch: String,
    pub gallery_revision: String,
    pub person_count: u32,
    pub cluster_count: u32,
    pub sample_count: u32,
    pub image_bytes: u64,
    pub image_limit_bytes: u64,
    pub unknown_retention_days: u32,
    pub models: Vec<EmbeddingSpace>,
    pub nodes: Vec<NodeSyncStatus>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSyncStatus {
    pub node_id: String,
    pub epoch: String,
    pub revision: String,
    pub acknowledged_at_ms: u64,
}
