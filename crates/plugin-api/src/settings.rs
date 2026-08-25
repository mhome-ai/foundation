use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::V1;
pub const STATUS: &str = "settings/status";
pub const UPDATE: &str = "settings/update";
pub const REVERT: &str = "settings/revert";
pub const RETRY: &str = "settings/retry";
pub const CHANGED_TARGET: &str = "/plugin/settings/changed";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StatusRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateRequest<T> {
    pub idempotency_key: String,
    pub section: String,
    pub expected_revision: u64,
    pub value: T,
}

impl<T> UpdateRequest<T> {
    pub fn new(idempotency_key: String, section: String, expected_revision: u64, value: T) -> Self {
        Self {
            idempotency_key,
            section,
            expected_revision,
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SectionCommandRequest {
    pub idempotency_key: String,
    pub section: String,
    pub expected_revision: u64,
}

impl SectionCommandRequest {
    pub fn new(idempotency_key: String, section: String, expected_revision: u64) -> Self {
        Self {
            idempotency_key,
            section,
            expected_revision,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChangedEvent {
    pub version: String,
    pub plugin_id: String,
    pub plugin_type: String,
    pub event_seq: u64,
    pub section: String,
    pub revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changed_event_accepts_additive_fields_without_internal_identity() {
        let event = serde_json::from_value::<ChangedEvent>(serde_json::json!({
            "version": "v1",
            "pluginId": "camera-1",
            "pluginType": "camera",
            "eventSeq": 7,
            "section": "recognition",
            "revision": 3,
            "futureField": true
        }))
        .unwrap();
        assert_eq!(event.plugin_id, "camera-1");
    }
}
