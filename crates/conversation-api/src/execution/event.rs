//! Durable Agent events and their delivery boundary.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::execution::{
    EventId, ExternalError, InvocationContext, MessageId, PreparedAction, RuntimeSnapshot,
    UsageSummary,
};

/// Event emitted by the Runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// A run has started or resumed.
    Started,
    /// The run requires user input before it can continue.
    InteractionRequired {
        /// Batch identifier answered by `SubmitInteraction`.
        batch_id: MessageId,
        /// Prepared actions awaiting one decision each.
        actions: Vec<PreparedAction>,
        /// Usage accumulated before Runtime suspended the run.
        usage: UsageSummary,
    },
    /// The run completed successfully.
    Completed {
        /// Final structured result.
        result: Value,
        /// Aggregated usage for the completed run.
        usage: UsageSummary,
    },
    /// The run failed definitively.
    Failed {
        /// Stable machine-readable failure code.
        code: String,
        /// Safe user-facing or diagnostic message.
        message: String,
        /// Usage settled before the failure, cancellation, or interruption.
        usage: UsageSummary,
    },
}

/// Best-effort, request-scoped observation that is not part of the durable outbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentObservation {
    /// Coarse progress for live presentation.
    Progress {
        /// Stable progress label.
        status: String,
    },
    /// Temporary assistant text emitted before a tool batch.
    CompanionText {
        /// Text shown while work continues.
        content: String,
        /// Whether a client should append rather than replace.
        append: bool,
    },
    /// A tool attempt is starting.
    ToolStarted {
        /// Model tool-call identifier.
        call_id: String,
        /// Stable tool name.
        tool_name: String,
    },
    /// A tool attempt produced a result.
    ToolResult {
        /// Model tool-call identifier.
        call_id: String,
        /// Stable tool name.
        tool_name: String,
        /// Structured result.
        result: Value,
        /// Whether execution failed.
        is_error: bool,
    },
    /// A validated model tool-call batch has been checkpointed for execution.
    ToolCallsScheduled {
        /// Tool names in model order.
        tool_names: Vec<String>,
    },
    /// A tool batch completed and its results are available to the next model step.
    ToolCallsCompleted {
        /// Number of completed tool calls.
        count: u32,
    },
    /// Optional diagnostic payload requested by the caller.
    DebugData {
        /// Diagnostic namespace.
        scope: String,
        /// Structured diagnostic value.
        payload: Value,
    },
}

/// Sequenced event persisted as part of a thread transition.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DurableEvent {
    /// Globally unique event identifier used for deduplication.
    pub id: EventId,
    /// Monotonic sequence within a thread.
    pub sequence: u64,
    /// Frozen routing, ownership, and run identity for this event.
    pub context: InvocationContext,
    /// Event payload.
    pub event: AgentEvent,
}

/// Delivery boundary for events that have already been durably committed.
#[async_trait]
pub trait EventPublisher: Send + Sync {
    /// Publishes one committed stable-boundary event with at-least-once delivery semantics.
    ///
    /// `snapshot` is the exact Runtime generation committed with `event`. Implementations must
    /// never reload a newer snapshot while encoding the event.
    async fn publish(
        &self,
        snapshot: &RuntimeSnapshot,
        event: &DurableEvent,
    ) -> Result<(), ExternalError>;
}

/// Best-effort observer for non-durable UI progress.
#[async_trait]
pub trait Observer: Send + Sync {
    /// Delivers one request-scoped observation.
    async fn observe(&self, context: &InvocationContext, observation: &AgentObservation);
}
