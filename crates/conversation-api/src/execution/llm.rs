//! Agent model port and execution policy, using the shared model contract.
use crate::execution::{ExternalError, InvocationContext};
use async_trait::async_trait;
pub use llm_api::{
    Completion, CompletionRequest, ContentPart, Continuation, FinishReason, INPUT_AUDIO,
    INPUT_FILE, INPUT_IMAGE, INPUT_VIDEO, Message, MessageRole, ModelCapabilities,
    ModelConstraints, ModelMode, ModelProfile, TokenUsage, ToolCall, UseCase,
    input_modality_for_kind, payload_input_modalities,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Tool schema exposed to an LLM.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Stable tool name.
    pub name: String,
    /// Human-readable behavior description.
    pub description: String,
    /// JSON Schema for tool input.
    pub input_schema: Value,
    /// Runtime policy for executing the tool.
    pub policy: ToolPolicy,
}

/// Side-effect class used by Runtime access policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEffect {
    /// Tool is safe to execute as a query.
    ReadOnly,
    /// Tool requires prepare/commit semantics.
    Mutating,
}

/// When a prepared tool operation requires a user decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalRequirement {
    /// Operation never pauses for approval.
    Never,
    /// Operation pauses only when the run uses interactive access.
    WhenInteractive,
    /// Operation always pauses, including full-access runs.
    Always,
}

impl ApprovalRequirement {
    /// Relative strength used when the Agent computes approval per invocation.
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Never => 0,
            Self::WhenInteractive => 1,
            Self::Always => 2,
        }
    }
}

/// Whether approval is fixed by the schema or strengthened after canonical preparation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalPolicy {
    /// Every invocation uses the same requirement.
    Fixed {
        /// Exact requirement for every invocation.
        requirement: ApprovalRequirement,
    },
    /// App Facade preparation chooses a requirement no weaker than this minimum.
    PerInvocation {
        /// Minimum requirement the prepared action may declare.
        minimum: ApprovalRequirement,
    },
}

impl ApprovalPolicy {
    /// Returns whether the prepared requirement is permitted by this declaration.
    #[must_use]
    pub const fn permits(self, requirement: ApprovalRequirement) -> bool {
        match self {
            Self::Fixed { requirement: fixed } => fixed.rank() == requirement.rank(),
            Self::PerInvocation { minimum } => requirement.rank() >= minimum.rank(),
        }
    }

    /// Returns whether every invocation is unconditionally approval-free.
    #[must_use]
    pub const fn is_fixed_never(self) -> bool {
        matches!(
            self,
            Self::Fixed {
                requirement: ApprovalRequirement::Never
            }
        )
    }
}

/// Runtime policy declared by a tool implementation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToolPolicy {
    /// Side-effect class of the tool.
    pub effect: ToolEffect,
    /// Approval rule for a concrete invocation.
    pub approval: ApprovalPolicy,
    /// Whether the Agent declares independent calls safe for parallel execution.
    pub parallel_safe: bool,
}

/// Aggregated model and tool usage for one Agent run.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct UsageSummary {
    /// Model requests attempted by Runtime.
    pub model_requests: u32,
    /// Requests whose provider returned token accounting.
    pub reported_model_requests: u32,
    /// Aggregated input tokens.
    pub input_tokens: u64,
    /// Aggregated output tokens.
    pub output_tokens: u64,
    /// Aggregated cached input tokens when any provider reported them.
    pub cached_input_tokens: Option<u64>,
    /// Aggregated reasoning output tokens when any provider reported them.
    pub reasoning_output_tokens: Option<u64>,
    /// Largest single-request input observed or estimated during the run.
    pub peak_input_tokens: u64,
    /// Aggregated provider-normalized billable credits when reported.
    pub credits: Option<u64>,
    /// Tool calls requested by models.
    pub tool_calls: u32,
    /// Number of prompt-window truncations across the main loop and derived workflows.
    pub window_truncations: u32,
    /// Number of prompt messages removed by window truncation.
    pub trimmed_messages: u64,
}

impl UsageSummary {
    /// Records that Runtime attempted one provider request.
    pub fn record_model_attempt(&mut self) {
        self.model_requests = self.model_requests.saturating_add(1);
    }

    /// Records normalized usage without incrementing the already-recorded attempt count.
    pub fn record_usage(&mut self, usage: TokenUsage, reported: bool) {
        if reported {
            self.reported_model_requests = self.reported_model_requests.saturating_add(1);
        }
        self.input_tokens = self.input_tokens.saturating_add(usage.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(usage.output_tokens);
        self.peak_input_tokens = self.peak_input_tokens.max(usage.input_tokens);
        add_optional(&mut self.cached_input_tokens, usage.cached_input_tokens);
        add_optional(
            &mut self.reasoning_output_tokens,
            usage.reasoning_output_tokens,
        );
        add_optional(&mut self.credits, usage.credits);
    }

    /// Records one completed model request while preserving missing telemetry.
    pub fn record_model_request(&mut self, usage: Option<TokenUsage>) {
        self.record_model_attempt();
        let Some(usage) = usage else {
            return;
        };
        self.record_usage(usage, true);
    }

    /// Records one deterministic prompt-history truncation.
    pub fn record_window_truncation(&mut self, removed_messages: usize) {
        self.window_truncations = self.window_truncations.saturating_add(1);
        self.trimmed_messages = self
            .trimmed_messages
            .saturating_add(u64::try_from(removed_messages).unwrap_or(u64::MAX));
    }
}

fn add_optional(total: &mut Option<u64>, value: Option<u64>) {
    if let Some(value) = value {
        *total = Some(total.unwrap_or(0).saturating_add(value));
    }
}

/// Model-specific failure that can retain usage returned before output validation failed.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
#[error("{error}")]
pub struct LlmFailure {
    /// Stable provider-neutral failure.
    pub error: ExternalError,
    /// Usage already reported by the provider, when available.
    pub usage: Option<TokenUsage>,
}

impl From<ExternalError> for LlmFailure {
    fn from(error: ExternalError) -> Self {
        Self { error, usage: None }
    }
}

/// LLM capability consumed by Runtime.
///
/// Provider discovery, credentials, model mappings, and provider-specific retries belong to the
/// implementation and are intentionally absent from this interface.
#[async_trait]
pub trait Llm: Send + Sync {
    /// Returns limits for the exact route frozen for one logical selector.
    async fn model_profile(
        &self,
        context: &InvocationContext,
        use_case: &UseCase,
        model_mode: &ModelMode,
    ) -> Result<ModelProfile, ExternalError>;

    /// Completes one logical model invocation.
    async fn complete(
        &self,
        context: &InvocationContext,
        request: CompletionRequest,
    ) -> Result<Completion, LlmFailure>;
}

impl ToolDefinition {
    pub fn model_definition(&self) -> llm_api::ToolDefinition {
        llm_api::ToolDefinition {
            name: self.name.clone(),
            description: self.description.clone(),
            input_schema: self.input_schema.clone(),
        }
    }
}
