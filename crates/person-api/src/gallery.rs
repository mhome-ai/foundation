use crate::EmbeddingSpace;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DescribeRequest {
    pub embedding_space: EmbeddingSpace,
    pub supported_policies: Vec<String>,
    pub proposed_policy: MatchPolicy,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeResponse {
    pub protocol: String,
    pub epoch: String,
    pub policy: MatchPolicy,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MatchPolicy {
    pub id: String,
    pub known_threshold: f32,
    pub unknown_threshold: f32,
    pub ambiguity_margin: f32,
}
impl Default for MatchPolicy {
    fn default() -> Self {
        Self {
            id: crate::MATCH_POLICY.into(),
            known_threshold: 0.62,
            unknown_threshold: 0.58,
            ambiguity_margin: 0.03,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GalleryRequest {
    pub embedding_space_id: String,
    pub epoch: Option<String>,
    pub after_revision: Option<String>,
    pub cursor: Option<String>,
}
/// A full snapshot or a coalesced delta. Each page belongs to exactly one revision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryResponse {
    pub epoch: String,
    pub revision: String,
    pub full: bool,
    pub policy: MatchPolicy,
    pub entries: Vec<GalleryEntry>,
    pub deleted_cluster_ids: Vec<String>,
    pub next_cursor: Option<String>,
    pub lease_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryEntry {
    pub cluster_id: String,
    pub person_id: Option<String>,
    pub centroid: Vec<f32>,
    pub exemplars: Vec<Vec<f32>>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GalleryAck {
    pub epoch: String,
    pub revision: String,
}

impl MatchPolicy {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.id != crate::MATCH_POLICY
            || !self.known_threshold.is_finite()
            || !self.unknown_threshold.is_finite()
            || !self.ambiguity_margin.is_finite()
            || !(0.0..=1.0).contains(&self.known_threshold)
            || !(0.0..=1.0).contains(&self.unknown_threshold)
            || !(0.0..=1.0).contains(&self.ambiguity_margin)
        {
            return Err("PERSON_UNSUPPORTED_MATCH_POLICY");
        }
        Ok(())
    }
}
