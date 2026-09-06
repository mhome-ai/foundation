use serde::{Deserialize, Serialize};

pub const SUBMIT_TARGET: &str = "/app/agent/audio/submit";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum AgentAudioTriggerKind {
    WakeWord,
    FollowUp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAudioTrigger {
    #[serde(rename = "type")]
    pub kind: AgentAudioTriggerKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake_word_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAudioSource {
    pub smart_speaker_id: String,
    pub input_device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAudioArtifact {
    pub uri: String,
    pub mime_type: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAudioSubmitRequest {
    pub request_id: String,
    pub source: AgentAudioSource,
    pub trigger: AgentAudioTrigger,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    pub audio: AgentAudioArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentAudioSubmitResponse {
    pub accepted: bool,
    pub interaction_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wake_word_trigger_has_a_stable_wire_shape() {
        let value = serde_json::to_value(AgentAudioTrigger {
            kind: AgentAudioTriggerKind::WakeWord,
            wake_word_id: Some("hello_meow_meow".to_string()),
        })
        .expect("serialize trigger");
        assert_eq!(value["type"], "wakeWord");
        assert_eq!(value["wakeWordId"], "hello_meow_meow");
    }
}
