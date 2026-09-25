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

/// Core classifies against the pre-learning gallery, then applies its learning policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum PersonDecision {
    Known {
        person_id: String,
        cluster_id: String,
        score: f32,
    },
    Stranger {
        cluster_id: Option<String>,
    },
    Uncertain,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObserveRequest {
    pub epoch: String,
    pub embedding_space_id: String,
    pub samples: Vec<SampleEvidence>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservationResult {
    pub decision: PersonDecision,
    pub receipt: SampleReceipt,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObserveResponse {
    pub epoch: String,
    pub results: Vec<ObservationResult>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decision_wire_contract_distinguishes_unknown_from_stranger() {
        let known = PersonDecision::Known {
            person_id: "person".into(),
            cluster_id: "cluster".into(),
            score: 0.9,
        };
        let value = serde_json::to_value(&known).unwrap();
        assert_eq!(value["status"], "known");
        assert_eq!(value["personId"], "person");
        assert!(value.get("person_id").is_none());
        assert_eq!(
            serde_json::from_value::<PersonDecision>(value).unwrap(),
            known
        );
        assert_ne!(
            serde_json::to_value(PersonDecision::Uncertain).unwrap()["status"],
            serde_json::to_value(PersonDecision::Stranger { cluster_id: None }).unwrap()["status"]
        );
    }
}
