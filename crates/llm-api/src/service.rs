use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const COMPLETE_TARGET: &str = "/llm/complete";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmGenerationOptions {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LlmCompleteRequest {
    #[serde(default)]
    pub constraints: crate::ModelConstraints,
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub messages: Vec<crate::Message>,
    #[serde(default)]
    pub tools: Option<Vec<crate::ToolDefinition>>,
    #[serde(default)]
    pub provider: Option<Value>,
    #[serde(default)]
    pub response_format: Option<Value>,
    #[serde(default)]
    pub options: LlmGenerationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmRouteInfo {
    pub provider: String,
    pub model: String,
    pub mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmCompleteResponse {
    pub message: crate::Message,
    pub finish_reason: crate::FinishReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<crate::TokenUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<LlmRouteInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_rejects_removed_top_level_generation_options() {
        assert!(
            serde_json::from_value::<LlmCompleteRequest>(serde_json::json!({
                "messages": [],
                "options": {},
                "temperature": 0.2
            }))
            .is_err()
        );
    }

    #[test]
    fn response_includes_normalized_usage() {
        let response = LlmCompleteResponse {
            usage: Some(crate::TokenUsage {
                input_tokens: 2,
                output_tokens: 3,
                ..crate::TokenUsage::default()
            }),
            ..LlmCompleteResponse::default()
        };
        assert_eq!(response.usage.unwrap().total_tokens(), 5);
    }
}

/// Finite audio is uploaded to the artifact service before transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AsrRequest {
    pub audio: artifact_api::MediaReference,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
}
