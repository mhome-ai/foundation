//! Canonical model invocation and continuation contract. No Agent or transport implementation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
mod continuation;
mod policy;
pub use continuation::Continuation;
pub use policy::{
    controls, input_modality_for_kind, input_modality_for_mime, normalize_capability_input,
    normalize_constraint_input, normalize_input_token, payload_input_modalities, resolve,
    BackendCapability, EffectiveGeneration, GenerationControls, GenerationParameters,
    GenerationSupport, ModelCapabilities, DEFAULT_MAX_OUTPUT_TOKENS, INPUT_AUDIO, INPUT_FILE,
    INPUT_IMAGE, INPUT_VIDEO, REASONING_EFFORT_LADDER,
};
pub use service::Image;

/// Logical model use case resolved by the deployment's LLM implementation.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UseCase(pub String);

/// Logical model mode selected by the product, independent of provider and model identifiers.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelMode(pub String);

/// Caller-declared requirements. Listed input modalities and true flags require confirmed
/// support; an empty `input` list and false flags impose no requirement.
/// Adapters must not infer or override these declarations from message content.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelConstraints {
    /// Closed vocabulary: `image`, `video`, `audio`, `file`. `text` and `vision` are rejected.
    #[serde(default)]
    pub input: Vec<String>,
    /// The model must support tool calls.
    pub tool_calling: bool,
    /// The model must support schema-constrained output.
    pub structured_output: bool,
}

/// Role of a message in a model conversation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum MessageRole {
    /// Runtime-supplied instruction.
    System,
    /// User-supplied input.
    #[default]
    User,
    /// Model output.
    Assistant,
    /// Tool execution output.
    Tool,
}

/// One typed part of a model message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ContentPart {
    /// Plain text content.
    Text {
        /// Text value.
        text: String,
    },
    /// Scope-owned content-addressed artifact managed by the application artifact service.
    Artifact {
        /// Canonical `meow-artifact://` URI.
        uri: String,
        /// MIME type copied from the validated artifact metadata.
        mime_type: String,
    },
    /// An image already materialized by the caller.
    Image { image: Image },
    /// A model-requested tool invocation.
    ToolCall(ToolCall),
    /// Result of an earlier tool invocation.
    ToolResult {
        /// Provider-neutral call identifier.
        call_id: String,
        /// Structured result payload.
        result: Value,
        /// Whether the tool execution failed.
        is_error: bool,
    },
}

/// Provider-neutral conversation message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Message {
    /// Speaker role.
    pub role: MessageRole,
    /// Ordered message content.
    pub content: Vec<ContentPart>,
    /// Private model continuation; never part of a user transcript.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<Continuation>,
}

/// A callable function exposed to the model. Execution policy belongs to the caller.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}
/// Tool call returned by an LLM.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolCall {
    /// Provider-neutral identifier used to correlate the result.
    pub id: String,
    /// Requested tool name.
    pub name: String,
    /// Structured arguments.
    pub arguments: Value,
}

/// One logical LLM invocation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Logical routing key resolved by the LLM adapter.
    pub use_case: UseCase,
    /// Logical quality/cost mode resolved by the LLM adapter adapter.
    pub model_mode: ModelMode,
    /// Conversation input.
    pub messages: Vec<Message>,
    /// Tools available to the model.
    pub tools: Vec<ToolDefinition>,
    /// Required model capabilities.
    pub constraints: ModelConstraints,
    /// Optional maximum number of output tokens.
    pub max_output_tokens: Option<u32>,
    /// Whether the caller requested sanitized diagnostic metadata.
    pub diagnostics: bool,
}

/// Provider-neutral limits for one frozen logical model route.
///
/// `profile_key` is opaque to Runtime. Implementations must change it whenever the concrete
/// model, tokenizer, or another input-serialization detail changes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModelProfile {
    /// Opaque stable identity of the frozen model and tokenizer route.
    pub profile_key: String,
    /// Maximum total context accepted by the concrete model.
    pub context_window_tokens: u32,
}

/// Why a model invocation stopped.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FinishReason {
    /// The model completed normally.
    #[default]
    Stop,
    /// The model requested one or more tools.
    ToolCalls,
    /// The configured output limit was reached.
    Length,
    /// The LLM adapter cannot map the provider result to a more specific reason.
    Other,
}

/// Normalized token accounting reported by an implementation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Input token count.
    pub input_tokens: u64,
    /// Output token count.
    pub output_tokens: u64,
    /// Input tokens served from a provider cache when reported.
    pub cached_input_tokens: Option<u64>,
    /// Reasoning output tokens when reported separately.
    pub reasoning_output_tokens: Option<u64>,
    /// Provider-normalized billable credits consumed by this request.
    pub credits: Option<u64>,
}

/// Provider-neutral completion returned to Runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Completion {
    /// Model response message.
    pub message: Message,
    /// Normalized finish reason.
    pub finish_reason: FinishReason,
    /// Normalized token accounting.
    pub usage: Option<TokenUsage>,
    /// Optional sanitized diagnostics without credentials or provider internals.
    pub diagnostics: Option<Value>,
}

pub mod service;

impl MessageRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::text(MessageRole::User, "")
    }
}
impl Message {
    pub fn text(role: MessageRole, text: impl Into<String>) -> Self {
        Self {
            role,
            content: vec![ContentPart::Text { text: text.into() }],
            continuation: None,
        }
    }
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl TokenUsage {
    pub fn total_tokens(self) -> u64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }
}
impl Message {
    pub fn with_images(mut self, images: impl IntoIterator<Item = Image>) -> Self {
        self.content
            .extend(images.into_iter().map(|image| ContentPart::Image { image }));
        self
    }
}

/// Offline normative schema for persisted model messages.
pub const MESSAGE_SCHEMA: &str = include_str!("../schema/message.v1.schema.json");
