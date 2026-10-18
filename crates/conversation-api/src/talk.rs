//! Bounded Pod presentation protocol. This is deliberately separate from Chat targets.
//! No histories, tool argument trees, execution commands or audio bytes cross this surface.
use crate::{ControlVersion, ConversationSurface, RunStatus};
use serde::{Deserialize, Serialize};

pub const VIEW_GET: &str = "/pod/talk/view/get";
pub const TURN_SUBMIT: &str = "/pod/talk/turn/submit";
pub const REQUEST_GET: &str = "/pod/talk/request/get";
pub const REQUEST_CANCEL: &str = "/pod/talk/request/cancel";
pub const REPLY_READ: &str = "/pod/talk/reply/read";
pub const INTERACTION_ANSWER: &str = "/pod/talk/interaction/answer";
pub const AUDIO_START: &str = "/pod/talk/audio/upload/start";
pub const AUDIO_TRANSCRIBE: &str = "/pod/talk/audio/transcribe";
pub const SPEECH_PREPARE: &str = "/pod/talk/speech/prepare";
pub const MEDIA_RESOLVE: &str = "/pod/talk/media/resolve";
pub const CHANGED: &str = "/pod/talk/changed";
pub const REQUEST_TARGETS: &[&str] = &[
    VIEW_GET,
    TURN_SUBMIT,
    REQUEST_GET,
    REQUEST_CANCEL,
    REPLY_READ,
    INTERACTION_ANSWER,
    AUDIO_START,
    AUDIO_TRANSCRIBE,
    SPEECH_PREPARE,
    MEDIA_RESOLVE,
];
pub fn is_request_target(target: &str) -> bool {
    REQUEST_TARGETS.contains(&target)
}

pub const FRAME_BYTES: usize = 64 * 1024;
pub const CHANGED_BYTES: usize = 2048;
pub const TEXT_BYTES: usize = 4096;
pub const REPLY_BYTES: usize = 16 * 1024;
pub const PENDING_BYTES: usize = 4096;
pub const ID_BYTES: usize = 256;
pub const RECORDING_BYTES: u32 = 960_000;
pub const AUDIO_DURATION_MS: u32 = 30_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Context {
    pub owner_id: String,
    pub scope_id: String,
    pub surface_id: ConversationSurface,
    pub thread_id: Option<String>,
    pub control_version: ControlVersion,
    /// Exact decimal u64; never decoded through an IEEE-754 number by a small client.
    pub history_version: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    pub submit: bool,
    pub answer: bool,
    pub transcribe: bool,
    pub speak: bool,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Unknown,
    Idle,
    Queued,
    Running,
    Pending,
    Settled,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Pending {
    pub thread_id: String,
    pub request_id: String,
    pub batch_id: String,
    pub text: String,
    pub action_count: u32,
    pub complete: bool,
    /// False when any omitted detail or client task prevents a safe whole-batch approval.
    pub can_approve: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reply {
    pub thread_id: String,
    pub request_id: String,
    pub message_id: String,
    /// Content identity, not a conversation/control ordering cursor.
    pub content_revision: String,
    /// UTF-8 byte offsets into the same immutable plain-text content.
    pub offset: u32,
    pub next_offset: Option<u32>,
    pub previous_offset: Option<u32>,
    pub text: String,
    pub has_unsupported_parts: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct View {
    pub context: Context,
    pub capabilities: Capabilities,
    pub phase: Phase,
    pub active_request_id: Option<String>,
    pub queue_count: u32,
    pub pending: Option<Pending>,
    pub reply: Option<Reply>,
    /// Latest run's authoritative terminal outcome in context.thread_id.
    /// Present only while phase is settled; never infer success from reply text.
    pub settlement: Option<Settlement>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Outcome {
    pub status: RunStatus,
    pub error_code: Option<String>,
    pub message: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settlement {
    pub request_id: String,
    /// InteractionRequired is a suspension, not a terminal settlement.
    pub outcome: Outcome,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestResult {
    pub context: Context,
    pub request_id: String,
    pub admission: Option<crate::RequestAdmissionState>,
    pub phase: Phase,
    pub outcome: Option<Outcome>,
    pub pending: Option<Pending>,
    pub reply: Option<Reply>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    pub request_id: String,
    pub thread_id: Option<String>,
    pub disposition: crate::TurnSubmitDisposition,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Changed {
    pub owner_id: String,
    pub scope_id: String,
    pub surface_id: ConversationSurface,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ViewGet {}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TurnSubmit {
    pub owner_id: String,
    pub request_id: String,
    pub text: String,
    pub occurred_at_unix_ms: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestGet {
    pub owner_id: String,
    pub request_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RequestCancel {
    pub owner_id: String,
    pub request_id: String,
    #[serde(default)]
    pub thread_id: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplyRead {
    pub owner_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub message_id: String,
    pub content_revision: String,
    pub offset: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InteractionAnswer {
    pub owner_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub batch_id: String,
    pub operation_id: String,
    pub text: String,
    pub occurred_at_unix_ms: i64,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnswerDisposition {
    Accepted,
    Clarify,
    Resolved,
    Expired,
    Unsupported,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AnswerReceipt {
    pub operation_id: String,
    pub disposition: AnswerDisposition,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
/// Starts PCM16 little-endian, mono 16 kHz ingestion. Raw samples are a MAN/TMP
/// FILE Artifact (application/octet-stream); the Talk adapter packages WAV for ASR.
/// Complete/cancel use the generic Artifact controls with the same verified Client.
pub struct AudioStart {
    pub owner_id: String,
    pub operation_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioTranscribe {
    pub owner_id: String,
    pub operation_id: String,
    pub uri: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transcript {
    pub operation_id: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpeechPrepare {
    pub owner_id: String,
    pub operation_id: String,
    pub thread_id: String,
    pub request_id: String,
    pub message_id: String,
    pub content_revision: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Speech {
    pub operation_id: String,
    pub uri: Option<String>,
    pub text_only_reason: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaResolve {
    pub owner_id: String,
    pub uri: String,
}
/// Finite binary HTTP grants reuse the Artifact contract and its pinned host identity.
pub type UploadGrant = artifact_api::StartArtifactUploadResponse;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedMedia {
    pub descriptor: artifact_api::ResolveArtifactResponse,
    pub sha256: String,
}
pub const SPEECH_TEXT_BYTES: usize = 4096;
pub const SPEECH_BYTES: usize = 1_536_044;
pub const SPEECH_DURATION_MS: u32 = 32_000;
/// Test the actual string-payload expansion, reserving space for the MWS envelope.
/// Implementations must still enforce the exact final encoded envelope at egress.
pub fn encoded_payload_fits(value: &impl Serialize, limit: usize) -> bool {
    serde_json::to_string(value)
        .ok()
        .and_then(|payload| serde_json::to_string(&payload).ok())
        .is_some_and(|quoted| quoted.len().saturating_add(1024) <= limit)
}

pub fn valid_identifier(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= ID_BYTES && !value.chars().any(char::is_control)
}

pub fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= TEXT_BYTES && !value.contains('\0')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn talk_targets_are_separate_from_chat() {
        for target in REQUEST_TARGETS {
            assert!(is_request_target(target));
            assert!(!crate::is_conversation_request_target(target));
        }
    }
    #[test]
    fn changed_and_escaped_text_fit_the_wire_budget() {
        let changed = Changed {
            owner_id: "core:test".into(),
            scope_id: "space".into(),
            surface_id: ConversationSurface::client_personal("user").unwrap(),
        };
        assert!(encoded_payload_fits(&changed, CHANGED_BYTES));
        assert!(!encoded_payload_fits(
            &"\"\\\n".repeat(REPLY_BYTES),
            FRAME_BYTES
        ));
    }
    #[test]
    fn input_limits_are_utf8_bytes_and_reject_nul() {
        assert!(valid_text("打开灯"));
        assert!(!valid_text(&"猫".repeat(TEXT_BYTES / 3 + 1)));
        assert!(!valid_text("yes\0no"));
        assert!(!valid_identifier("request\nother"));
    }
}
