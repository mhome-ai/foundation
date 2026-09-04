use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const COMPLETE_TARGET: &str = "/llm/complete";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl Image {
    #[must_use]
    pub fn from_base64(base64: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            base64: Some(base64.into()),
            mime_type: Some(mime_type.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn from_url(url: impl Into<String>) -> Self {
        Self {
            url: Some(url.into()),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn resolved_mime_type(&self) -> Option<String> {
        let mime_type = self
            .mime_type
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())?;
        let normalized = mime_type.to_ascii_lowercase();
        Some(match normalized.as_str() {
            "image/jpg" => "image/jpeg".to_string(),
            _ => normalized,
        })
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.url
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
            && self
                .base64
                .as_ref()
                .is_none_or(|value| value.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ChatToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<Image>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ChatToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

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
    pub use_case: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub tools: Option<Value>,
    #[serde(default)]
    pub provider: Option<Value>,
    #[serde(default)]
    pub response_format: Option<Value>,
    #[serde(default)]
    pub options: LlmGenerationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LlmTokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cached_input_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_output_tokens: Option<u64>,
}

impl LlmTokenUsage {
    #[must_use]
    pub fn total_tokens(self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
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
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ProviderToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<LlmTokenUsage>,
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
            usage: Some(LlmTokenUsage {
                input_tokens: 2,
                output_tokens: 3,
                ..LlmTokenUsage::default()
            }),
            ..LlmCompleteResponse::default()
        };
        assert_eq!(response.usage.unwrap().total_tokens(), 5);
    }
}
