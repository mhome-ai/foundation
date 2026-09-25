//! Core matching and archive data; never negotiated by evidence producers.
use serde::{Deserialize, Serialize};
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
#[serde(rename_all = "camelCase")]
pub struct GalleryEntry {
    /// Competes during matching, but cannot establish a recognized identity yet.
    #[serde(default)]
    pub provisional: bool,
    pub cluster_id: String,
    pub person_id: Option<String>,
    pub centroid: Vec<f32>,
    pub exemplars: Vec<Vec<f32>>,
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
