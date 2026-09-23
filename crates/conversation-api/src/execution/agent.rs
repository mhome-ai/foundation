//! Commands accepted by the transport-independent Runtime.

use serde::{Deserialize, Serialize};

use crate::execution::{InvocationContext, Message, MessageId, ModelMode, RunId, UseCase};

/// Mutation policy for tools that can change application state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    /// Operations requiring approval are denied without creating an interaction.
    /// Operations requiring no approval remain available.
    NoAccess,
    /// Mutating tools pause after prepare and require a user decision.
    #[default]
    Interactive,
    /// Mutating tools may be committed without a user interaction.
    FullAccess,
}

/// Per-run policy supplied by a deployment rather than selected through conditional compilation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunOptions {
    /// Logical LLM route selected by the deployment.
    pub use_case: UseCase,
    /// Logical quality/cost mode resolved by the deployment without exposing a provider or model ID.
    pub model_mode: ModelMode,
    /// Tool mutation policy.
    pub access_mode: AccessMode,
    /// Whether this run may expose any tools to the model.
    pub allow_tools: bool,
    /// Maximum LLM/tool loop steps requested by the caller.
    pub max_steps: u32,
    /// Optional maximum normalized credits the run may consume.
    pub credit_budget: Option<u64>,
    /// Requests additional sanitized diagnostic observations when enabled.
    pub debug: bool,
}

/// Decision for one prepared action in a pending interaction batch.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InteractionDecision {
    /// Prepared action identifier.
    pub action_id: crate::execution::ActionId,
    /// Whether Runtime should commit or reject the action.
    pub proceed: bool,
}

/// User decisions submitted to a waiting run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    /// Batch identifier emitted by `InteractionRequired`.
    pub batch_id: MessageId,
    /// One decision for every prepared action in the batch.
    pub decisions: Vec<InteractionDecision>,
}

/// Durable request waiting to enter the Runtime execution loop.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct QueuedRun {
    /// Invocation identity frozen when the request is accepted.
    pub context: InvocationContext,
    /// User-supplied multimodal message.
    pub message: Message,
    /// Runtime policy frozen when the request is accepted.
    pub options: RunOptions,
}

impl QueuedRun {
    /// Returns the stable run identifier used for queue idempotency.
    #[must_use]
    pub fn run_id(&self) -> &RunId {
        &self.context.run_id
    }
}

/// Semantic commands understood by the Runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum AgentCommand {
    /// Enqueues a new user message.
    Enqueue {
        /// Invocation identity and ownership.
        context: InvocationContext,
        /// User-supplied multimodal message.
        message: Message,
        /// Runtime policy for this run.
        options: RunOptions,
    },
    /// Supplies a response to a pending interaction.
    SubmitInteraction {
        /// Invocation identity and ownership.
        context: InvocationContext,
        /// Submitted interaction.
        interaction: Interaction,
    },
    /// Requests cancellation of the current run.
    Cancel {
        /// Invocation identity and ownership.
        context: InvocationContext,
    },
}
