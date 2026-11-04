//! Ephemeral input preparation. None of these operations admits a business turn.
use super::Context;
use serde::{Deserialize, Serialize};

pub const INPUT_BEGIN: &str = "/pod/talk/input/begin";
pub const INPUT_APPEND: &str = "/pod/talk/input/append";
pub const INPUT_ASSESS: &str = "/pod/talk/input/assess";
pub const INPUT_CANCEL: &str = "/pod/talk/input/cancel";
pub const INPUT_LIFETIME_MS: u32 = 60_000;
pub const INPUT_SEGMENTS: usize = 32;
/// Integers must be exactly representable by JSON clients.
pub const INPUT_EPOCH_MAX: u64 = (1_u64 << 53) - 1;

pub fn is_input_target(target: &str) -> bool {
    matches!(
        target,
        INPUT_BEGIN | INPUT_APPEND | INPUT_ASSESS | INPUT_CANCEL
    )
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputLocale {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputBinding {
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputBegin {
    pub owner_id: String,
    pub input_id: String,
    pub input_epoch: u64,
    pub context: Context,
    pub locale: InputLocale,
    /// Exact configured wake phrase; stripped deterministically on the first segment only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wake_phrase: Option<String>,
    /// Current settled reply whose local playback yielded to this input.
    /// Mutually exclusive with wake_phrase; this is not permission to resume.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interruption: Option<InputInterruption>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputInterruption {
    pub thread_id: String,
    pub request_id: String,
    pub message_id: String,
    pub content_revision: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputInterruptionIntent {
    Task,
    Backchannel,
    IrrelevantSound,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputStarted {
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
    pub context: Context,
    pub max_audio_duration_ms: u32,
    /// Remaining absolute lifetime; replays never extend it.
    pub expires_in_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputAppend {
    pub owner_id: String,
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
    pub operation_id: String,
    pub uri: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputTranscript {
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
    /// Ordered original ASR segments, including corrections. Never rewritten.
    pub text: String,
    pub audio_duration_ms: u32,
    /// Raw ASR was empty or the required first wake prefix could not be confirmed.
    /// The whole draft is revoked, including earlier text.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub unheard: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputAssess {
    pub owner_id: String,
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
    /// Incomplete input must clarify at the user's finish / bounded pause.
    pub finalize: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InputDecision {
    Ready,
    Wait,
    Clarify,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputAssessment {
    pub input_id: String,
    pub input_epoch: u64,
    pub revision: u32,
    pub decision: InputDecision,
    pub text: String,
    pub clarification: Option<String>,
    /// Present only for interruption drafts. Only Task + Ready may submit;
    /// benign intents still require the client's current-reply/acoustic check.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interruption_intent: Option<InputInterruptionIntent>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputCancel {
    pub owner_id: String,
    pub input_id: String,
    pub input_epoch: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InputCancelled {
    pub input_id: String,
    pub input_epoch: u64,
    /// False after admission claimed first; recover/cancel the original business request.
    pub cancelled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_is_explicit_and_does_not_change_legacy_submit() {
        let old = r#"{"ownerId":"core:one","requestId":"r","text":"hello","occurredAtUnixMs":0}"#;
        let submit: super::super::TurnSubmit = serde_json::from_str(old).unwrap();
        assert!(submit.input.is_none());
        assert_eq!(
            serde_json::to_value(submit).unwrap(),
            serde_json::from_str::<serde_json::Value>(old).unwrap()
        );
        for target in [INPUT_BEGIN, INPUT_APPEND, INPUT_ASSESS, INPUT_CANCEL] {
            assert!(super::super::is_request_target(target));
            assert!(is_input_target(target));
        }
        assert!(!is_input_target(super::super::TURN_SUBMIT));
    }

    #[test]
    fn finite_decisions_and_strict_input_do_not_accept_extra_authority() {
        assert!(serde_json::from_str::<InputDecision>("\"execute\"").is_err());
        assert!(
            serde_json::from_str::<InputBinding>(
                r#"{"inputId":"i","inputEpoch":1,"revision":1,"scopeId":"other"}"#
            )
            .is_err()
        );
    }
}
