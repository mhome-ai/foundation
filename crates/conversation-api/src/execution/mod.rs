//! Consumer-owned contracts for the Agent runtime.
//!
//! This crate contains transport-neutral domain types and ports. Implementations live in the local
//! `MeowCore` composition or the private `agent-cloud` composition.

pub mod agent;
pub mod context;
pub mod error;
pub mod event;
pub mod facade;
pub mod id;
pub mod llm;
pub mod operational_context;
pub mod storage;

pub use crate::ConversationSurface;
pub use agent::{
    AccessMode, AgentCommand, Interaction, InteractionDecision, QueuedRun, RunOptions,
};
pub use artifact_api::{ARTIFACT_URL_PREFIX, ArtifactKind, ArtifactMetadata, ArtifactReference};
pub use context::{Actor, InvocationContext, Scope};
pub use error::{ExternalError, ExternalErrorKind};
pub use event::{AgentEvent, AgentObservation, DurableEvent, EventPublisher, Observer};
pub use facade::{
    AppFacade, ClientTask, FacadeRequest, FacadeResult, InteractionKind, InteractionPreview,
    InteractionTone, PreparedAction, PreviewDetail, PreviewValue,
};
pub use id::{
    ActionId, ClientId, EventId, MessageId, OperationId, RunId, ScopeId, TenantId, ThreadId, UserId,
};
pub use llm::{
    ApprovalPolicy, ApprovalRequirement, Completion, CompletionRequest, ContentPart, Continuation,
    FinishReason, Llm, LlmFailure, Message, MessageRole, ModelConstraints, ModelMode, ModelProfile,
    TokenUsage, ToolCall, ToolDefinition, ToolEffect, ToolPolicy, UsageSummary, UseCase,
};
pub use operational_context::{
    ContextGeneration, ContextInvalidation, ContextSource, OperationalContext,
    OperationalContextDelta,
};
pub use storage::{
    Checkpoint, ClaimNextOutcome, CommitOutcome, HistoryMutation, QueueSnapshot, QueueStore,
    Revision, RunControl, RunCoordinator, RunLease, RuntimeSnapshot, StoreError, ThreadCommit,
    ThreadKey, ThreadRecord, ThreadStore,
};

pub mod wire;
