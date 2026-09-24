use crate::{EmbeddingSpace, Person};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SampleEvidence {
    pub id: String,
    pub source_device_id: String,
    pub source_run_id: String,
    pub source_track_id: String,
    pub captured_at_ms: u64,
    pub vector: Vec<f32>,
    pub quality: f32,
    pub image_base64: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitSamplesRequest {
    pub epoch: String,
    pub embedding_space_id: String,
    pub samples: Vec<SampleEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSamplesResponse {
    pub epoch: String,
    pub revision: String,
    pub results: Vec<SampleReceipt>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SampleReceipt {
    pub id: String,
    pub outcome: SampleOutcome,
    pub cluster_id: Option<String>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SampleOutcome {
    Active,
    Pending,
    Duplicate,
    Rejected,
    Deferred,
}
/// A bounded, resumable legacy import. Identity is namespaced by authenticated producer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyImportRequest {
    pub grant_token: String,
    pub epoch: String,
    pub embedding_space: EmbeddingSpace,
    pub legacy_cluster_id: String,
    pub legacy_person: Option<Person>,
    pub samples: Vec<SampleEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyFinishRequest {
    pub grant_token: String,
    pub epoch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyOfferRequest {
    pub person_count: u32,
    pub sample_count: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyOfferResponse {
    pub epoch: String,
    pub grant_token: Option<String>,
    pub state: String,
}

/// A producer reports a bounded diagnostic; biometric evidence is never part of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyReportRequest {
    pub error: Option<String>,
}
