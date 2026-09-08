//! Internal Agent Gateway ingress. Authentication supplies tenant, scope and Node identity.
use serde::{Deserialize, Serialize};

pub const SUBMIT_TARGET: &str = "/agent/submit";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioReference {
    pub uri: String,
    pub mime_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum SessionSelection {
    New { operation_id: String },
    Existing { thread_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", deny_unknown_fields)]
pub enum Input { Text { text: String }, Audio { audio: AudioReference } }

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputFormat { #[default] Text, Audio }

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventSelection { #[default] Final, Conversation }

/// Optional live conversation events, in addition to the terminal reply. These are
/// best-effort; the canonical event's versions support duplicate/stale detection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConversationEventRequest {
    pub request_id: String,
    pub endpoint_id: String,
    pub event: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputOptions {
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub events: EventSelection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitRequest {
    pub request_id: String,
    pub endpoint_id: String,
    pub session: SessionSelection,
    pub input: Input,
    #[serde(default)]
    pub output: OutputOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmitResponse {
    pub request_id: String,
    pub accepted: bool,
    pub expires_at_unix_ms: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ResultStatus { Completed, Failed, Cancelled, RequiresAction }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplyRequest {
    pub delivery_id: String,
    pub request_id: String,
    pub thread_id: String,
    pub endpoint_id: String,
    pub status: ResultStatus,
    pub expires_at_unix_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioReference>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_event_envelope_preserves_canonical_event_and_rejects_extra_identity() {
        let value = serde_json::json!({
            "requestId": "request", "endpointId": "endpoint",
            "event": {"type": "messageDelta", "threadId": "thread", "version": 4}
        });
        let event: ConversationEventRequest = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(event).unwrap(), value);
        let mut forged = value;
        forged["nodeId"] = serde_json::json!("other-node");
        assert!(serde_json::from_value::<ConversationEventRequest>(forged).is_err());
    }

    #[test]
    fn generic_input_defaults_to_final_text_and_has_no_spoofable_caller_identity() {
        let value=serde_json::json!({"requestId":"r","endpointId":"speaker", "session":{"type":"new","operationId":"wake"},"input":{"type":"text","text":"hello"}});
        let request:SubmitRequest=serde_json::from_value(value.clone()).unwrap();
        assert_eq!(request.output,OutputOptions { format:OutputFormat::Text, events:EventSelection::Final });
        let mut forged=value; forged["nodeId"]=serde_json::json!("other");
        assert!(serde_json::from_value::<SubmitRequest>(forged).is_err());
    }

    #[test]
    fn audio_follow_up_and_failure_without_audio_round_trip() {
        let request=SubmitRequest { request_id:"r".into(),endpoint_id:"speaker".into(),
          session:SessionSelection::Existing { thread_id:"thread".into() },
          input:Input::Audio { audio:AudioReference { uri:"meow-artifact://recording".into(),mime_type:"audio/ogg".into(),duration_ms:Some(1000) } },
          output:OutputOptions {format:OutputFormat::Audio,events:EventSelection::Final} };
        let value=serde_json::to_value(&request).unwrap();
        assert_eq!(value["session"]["threadId"],"thread");
        assert_eq!(serde_json::from_value::<SubmitRequest>(value).unwrap(),request);
        let reply=ReplyRequest {delivery_id:"d".into(),request_id:"r".into(),thread_id:"thread".into(),endpoint_id:"speaker".into(),status:ResultStatus::Failed,expires_at_unix_ms:1,text:None,audio:None};
        assert_eq!(serde_json::from_value::<ReplyRequest>(serde_json::to_value(&reply).unwrap()).unwrap(),reply);
    }
}
