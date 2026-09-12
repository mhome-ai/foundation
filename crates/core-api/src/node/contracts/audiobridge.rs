use serde::{Deserialize, Serialize};

pub const VERSION: &str = crate::node::V1;
pub const NODE_TYPE: &str = "audiobridge";
pub const APP_TARGET_PREFIX: &str = "/app/plugin/audiobridge/";
pub const RUNTIME_TARGET_PREFIX: &str = "/audiobridge/app/";
pub const DEVICE_SNAPSHOT: &str = "device/snapshot";
pub const DEVICE_REFRESH: &str = "device/refresh";
pub const DEVICE_PAIR: &str = "device/pair";
pub const DEVICE_UNPAIR: &str = "device/unpair";
pub const DEVICE_TEST: &str = "device/test";
pub const DEVICE_LISTEN_SET: &str = "device/listen";
pub const SMART_SPEAKER_SNAPSHOT: &str = "smart-speaker/snapshot";
pub const SMART_SPEAKER_CREATE: &str = "smart-speaker/create";
pub const SMART_SPEAKER_UPDATE: &str = "smart-speaker/update";
pub const SMART_SPEAKER_REMOVE: &str = "smart-speaker/remove";
pub const AGENT_REPLY_TARGET: &str = "/audiobridge/agent/reply";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EmptyRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceRequest {
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceListenSetRequest {
    pub device_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AudioDeviceFeature {
    Speaker,
    Microphone,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MicrophonePermission {
    NotDetermined,
    Granted,
    Denied,
    Restricted,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeDevice {
    pub device_id: String,
    pub display_name: String,
    pub transport: String,
    pub features: Vec<AudioDeviceFeature>,
    pub online: bool,
    pub listen_enabled: bool,
    pub listen_runtime: ListenRuntimeStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ListenRuntimeState {
    Disabled,
    Starting,
    Listening,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ListenRuntimeStatus {
    pub state: ListenRuntimeState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AudioBridgeCandidate {
    pub device_id: String,
    pub display_name: String,
    pub transport: String,
    pub features: Vec<AudioDeviceFeature>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSnapshot {
    pub microphone_permission: MicrophonePermission,
    #[serde(default)]
    pub devices: Vec<AudioBridgeDevice>,
    #[serde(default)]
    pub candidates: Vec<AudioBridgeCandidate>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SmartSpeakerRuntimeState {
    Disabled,
    Starting,
    Listening,
    PlaybackMuted,
    Capturing,
    Submitting,
    WaitingForReply,
    ReplyPlaying,
    FollowUpListening,
    InputOffline,
    OutputOffline,
    PermissionDenied,
    ModelError,
    RuntimeError,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeaker {
    pub id: String,
    pub display_name: String,
    pub input_device_id: String,
    pub output_device_id: String,
    pub enabled: bool,
    pub online: bool,
    pub state: SmartSpeakerRuntimeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeakerSnapshot {
    #[serde(default)]
    pub smart_speakers: Vec<SmartSpeaker>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeakerCreateRequest {
    pub display_name: String,
    pub input_device_id: String,
    pub output_device_id: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeakerUpdateRequest {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_device_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeakerRemoveRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SmartSpeakerMutationResponse {
    pub changed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smart_speaker: Option<SmartSpeaker>,
}

pub use super::agent_gateway::ReplyRequest as AgentReplyRequest;

const fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingResponse {
    pub changed: bool,
    pub device: AudioBridgeDevice,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_and_rust_audiobridge_contract_agree() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../../manifest/core.v1.json")).unwrap();
        let contract = &manifest["protocols"]["audioBridgePlugin"];
        assert_eq!(contract["version"], VERSION);
        assert_eq!(contract["nodeType"], NODE_TYPE);
        assert_eq!(contract["appTargetPrefix"], APP_TARGET_PREFIX);
        assert_eq!(contract["runtimeTargetPrefix"], RUNTIME_TARGET_PREFIX);
        assert_eq!(contract["routes"]["deviceSnapshot"], DEVICE_SNAPSHOT);
        assert_eq!(contract["routes"]["deviceRefresh"], DEVICE_REFRESH);
        assert_eq!(contract["routes"]["devicePair"], DEVICE_PAIR);
        assert_eq!(contract["routes"]["deviceUnpair"], DEVICE_UNPAIR);
        assert_eq!(contract["routes"]["deviceTest"], DEVICE_TEST);
        assert_eq!(contract["routes"]["deviceListenSet"], DEVICE_LISTEN_SET);
        assert_eq!(
            contract["routes"]["smartSpeakerSnapshot"],
            SMART_SPEAKER_SNAPSHOT
        );
        assert_eq!(
            contract["routes"]["smartSpeakerCreate"],
            SMART_SPEAKER_CREATE
        );
        assert_eq!(
            contract["routes"]["smartSpeakerUpdate"],
            SMART_SPEAKER_UPDATE
        );
        assert_eq!(
            contract["routes"]["smartSpeakerRemove"],
            SMART_SPEAKER_REMOVE
        );
        assert_eq!(contract["agentReplyTarget"], AGENT_REPLY_TARGET);
        assert_eq!(
            manifest["protocols"]["agentGateway"]["submitTarget"],
            crate::node::contracts::agent_gateway::SUBMIT_TARGET
        );
    }

    #[test]
    fn public_contract_does_not_expose_platform_identity() {
        let value = serde_json::to_value(AudioBridgeDevice {
            device_id: "device-1".to_string(),
            display_name: "Living Room Speaker".to_string(),
            transport: "bluetooth".to_string(),
            features: vec![AudioDeviceFeature::Speaker, AudioDeviceFeature::Microphone],
            online: true,
            listen_enabled: false,
            listen_runtime: ListenRuntimeStatus {
                state: ListenRuntimeState::Disabled,
                reason: None,
            },
        })
        .expect("serialize device");
        assert!(value.get("platformKey").is_none());
        assert!(value.get("endpointId").is_none());
        assert_eq!(value["listenRuntime"]["state"], "disabled");
        assert_eq!(
            value["features"],
            serde_json::json!(["speaker", "microphone"])
        );
    }

    #[test]
    fn runtime_failures_are_not_model_or_device_failures() {
        assert_eq!(
            serde_json::to_value(SmartSpeakerRuntimeState::RuntimeError).unwrap(),
            "runtimeError"
        );
        let status = ListenRuntimeStatus {
            state: ListenRuntimeState::Failed,
            reason: Some("input_stream_unavailable".into()),
        };
        let value = serde_json::to_value(&status).unwrap();
        assert_eq!(value["state"], "failed");
        assert_eq!(
            serde_json::from_value::<ListenRuntimeStatus>(value).unwrap(),
            status
        );
    }

    #[test]
    fn device_snapshot_is_the_direct_response_payload() {
        let value = serde_json::to_value(DeviceSnapshot {
            microphone_permission: MicrophonePermission::NotDetermined,
            devices: Vec::new(),
            candidates: Vec::new(),
        })
        .expect("serialize device snapshot");
        assert!(value.get("devices").is_some());
        assert!(value.get("candidates").is_some());
        assert!(value.get("snapshot").is_none());
    }

    #[test]
    fn smart_speaker_contract_keeps_input_and_output_bindings_explicit() {
        let value = serde_json::to_value(SmartSpeakerCreateRequest {
            display_name: "Kitchen".to_string(),
            input_device_id: "microphone-1".to_string(),
            output_device_id: "speaker-1".to_string(),
            enabled: true,
        })
        .expect("serialize smart speaker request");
        assert_eq!(value["inputDeviceId"], "microphone-1");
        assert_eq!(value["outputDeviceId"], "speaker-1");
        assert_eq!(value["enabled"], true);
    }
}
