//! Versioned transport contract for Agent commands, events, and App Facade calls.
//!
//! This crate defines envelopes but no WebSocket client, authentication, reconnect policy, or
//! server. Transport implementations belong to their deployment repository.

/// Golden enqueue envelope used by deployment and Runtime conformance tests.
pub const ENQUEUE_FIXTURE: &str = include_str!("../../../fixtures/execution/enqueue.v1.json");

use crate::execution::{
    AgentCommand, AgentObservation, ContextGeneration, ConversationSurface, DurableEvent, EventId,
    FacadeRequest, FacadeResult, InvocationContext, ModelMode, PreparedAction, Revision, RunId,
    Scope, ThreadId, UseCase,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current protocol major version.
pub const CURRENT_VERSION: u16 = 1;

/// Ephemeral cloud execution identity assigned by Lion.
///
/// Both values belong to the transport boundary and never enter the provider-neutral Runtime.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DispatchBinding {
    /// Unique identity for one execution attempt.
    pub dispatch_id: String,
    /// Authenticated Agent WebSocket session selected for the attempt.
    pub agent_session_id: String,
}

/// Immutable object-store reference to one Runtime checkpoint generation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckpointReference {
    /// Object key inside the configured checkpoint bucket.
    pub object_key: String,
    /// Lowercase hexadecimal SHA-256 of the checkpoint bytes.
    pub sha256: String,
    /// Runtime checkpoint format understood by `agent-rust`.
    pub format_version: u32,
}

/// Metadata shared by every wire message.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Metadata {
    /// Protocol major version.
    pub version: u16,
    /// Globally unique message identifier.
    pub message_id: String,
    /// Identifier correlating a request and its response or emitted events.
    pub correlation_id: String,
    /// Optional identifier of the message that caused this one.
    pub causation_id: Option<String>,
    /// Sender timestamp in Unix milliseconds.
    pub sent_at_unix_ms: u64,
    /// Execution-attempt and Agent-session binding.
    pub dispatch: DispatchBinding,
}

/// Request to execute an App Facade call over a transport.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
pub enum AppFacadeRequest {
    /// Executes a direct App Facade request.
    Invoke {
        /// Authenticated invocation context.
        context: InvocationContext,
        /// Canonical Agent-selected request.
        request: FacadeRequest,
    },
    /// Prepares a mutating operation.
    Prepare {
        /// Authenticated invocation context.
        context: InvocationContext,
        /// Canonical Agent-selected request.
        request: FacadeRequest,
    },
    /// Commits a previously prepared operation.
    Commit {
        /// Authenticated invocation context.
        context: InvocationContext,
        /// Prepared operation and idempotency metadata.
        action: PreparedAction,
    },
    /// Rejects a previously prepared operation.
    Reject {
        /// Authenticated invocation context.
        context: InvocationContext,
        /// Prepared operation and idempotency metadata.
        action: PreparedAction,
    },
}

/// Immediate response to command admission for the selected execution attempt.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandResponse {
    /// Command was admitted; final state is delivered through events.
    Accepted {
        /// Authenticated context of the admitted command.
        context: InvocationContext,
        /// Stable admission disposition.
        disposition: AdmissionDisposition,
        /// Queue revision when the command mutated the admission queue.
        queue_revision: Option<Revision>,
    },
    /// Command was rejected before admission.
    Rejected {
        /// Authenticated context of the rejected command.
        context: InvocationContext,
        /// Stable machine-readable error code.
        code: String,
        /// Safe diagnostic message.
        message: String,
    },
}

/// Stable wire representation of command admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdmissionDisposition {
    /// A Lion-dispatched request entered the Agent process's execution queue.
    Queued,
    /// Interaction decisions are durable and ready to resume.
    InteractionReady,
    /// Cancellation has been signaled to the live Agent process.
    CancelRequested,
}

/// Routed best-effort observation sent outside the durable event outbox.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObservationMessage {
    /// Authenticated invocation and routing context.
    pub context: InvocationContext,
    /// Request-scoped progress payload.
    pub observation: AgentObservation,
}

/// Terminal transport failure for an admitted execution that could not produce a checkpoint event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExecutionFailure {
    pub context: InvocationContext,
    pub code: String,
    pub message: String,
}

/// Acknowledges one durable event after the receiver has applied or deduplicated it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EventAck {
    pub event_id: EventId,
    /// Checkpoint reference Lion actually selected while applying (or deduplicating) the event.
    pub checkpoint: CheckpointReference,
}

/// Lion-owned archive GC request for all Runtime checkpoints under one private thread.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CheckpointPurgeRequest {
    pub purge_id: String,
    pub conversation_key: String,
    pub scope: Scope,
    pub surface_id: ConversationSurface,
    pub thread_id: ThreadId,
}

/// Result of one idempotent checkpoint-prefix purge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CheckpointPurgeAck {
    Deleted {
        purge_id: String,
        conversation_key: String,
        thread_id: ThreadId,
        deleted_objects: u32,
    },
    Failed {
        purge_id: String,
        conversation_key: String,
        thread_id: ThreadId,
        code: String,
        message: String,
    },
}

/// Model facts captured by the deployment with the desired route parameters.
/// This snapshot belongs to admission, not to the provider-neutral LLM request.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmModelSnapshot {
    pub capabilities: llm_api::ModelCapabilities,
    pub generation_support: LlmGenerationSupport,
}

/// Catalog parameter support travels with the plan so a run never resolves against a
/// catalog that changed after admission. Resolution itself belongs to `llm_api::resolve`.
pub use llm_api::GenerationSupport as LlmGenerationSupport;

/// One concrete cloud model route frozen by the deployment before command admission.
/// The generation fields are the deployment's desired preferences, not resolved values:
/// the runtime resolves them once against `model_snapshot` and the backend protocol.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmRoute {
    pub backend: String,
    pub profile: String,
    pub model: String,
    pub provider_preferences: Vec<String>,
    pub temperature: Option<f64>,
    pub cache_control: Option<bool>,
    pub reasoning_effort: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fast_mode: Option<bool>,
    /// Total context window of the exact frozen model route.
    pub context_window_tokens: u32,
    /// Required immutable facts; old plans must not silently resolve against a new catalog.
    pub model_snapshot: LlmModelSnapshot,
}

/// One logical selector and its concrete deployment route.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmRouteBinding {
    pub use_case: UseCase,
    pub model_mode: ModelMode,
    pub revision: Option<String>,
    pub route: LlmRoute,
}

/// Complete deployment-resolved LLM route set bound immutably to one admitted run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LlmExecutionPlan {
    pub routes: Vec<LlmRouteBinding>,
}

impl LlmExecutionPlan {
    /// Selects the exact frozen route for one provider-neutral Runtime request.
    #[must_use]
    pub fn route_for(
        &self,
        use_case: &UseCase,
        model_mode: &ModelMode,
    ) -> Option<&LlmRouteBinding> {
        self.routes
            .iter()
            .find(|item| item.use_case == *use_case && item.model_mode == *model_mode)
    }
}

/// Transport admission request. Environment bindings are consumed by the composition root and do
/// not enter the provider-neutral Runtime command.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub command: AgentCommand,
    /// Effective prompt-context generation selected by Lion for this dispatch.
    pub context_generation: Option<ContextGeneration>,
    pub llm_plan: Option<LlmExecutionPlan>,
    /// Stable Runtime checkpoint selected by Lion, absent for a new thread.
    pub base_checkpoint: Option<CheckpointReference>,
    /// Attempt already terminalized by Lion and settled silently before new work starts.
    pub abandoned_run_id: Option<RunId>,
}

/// Checkpoint-coupled Runtime event. Lion commits the generation before projecting the event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventMessage {
    pub event: DurableEvent,
    pub checkpoint: CheckpointReference,
}

/// Result of an App Facade request transported back to Runtime.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AppFacadeResponse {
    /// Read-only or committed operation result.
    Result {
        /// Structured App Facade result.
        result: FacadeResult,
    },
    /// Prepared operation awaiting a commit or rejection.
    Prepared {
        /// Prepared action.
        action: PreparedAction,
    },
    /// Rejection completed successfully.
    Rejected,
    /// Stable error safe to expose across the transport.
    Failed {
        /// Machine-readable category.
        code: String,
        /// Safe diagnostic message.
        message: String,
        /// Optional retry delay.
        retry_after_ms: Option<u64>,
    },
}

/// Payload families carried by the versioned envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "family", content = "payload", rename_all = "snake_case")]
pub enum Payload {
    /// Command entering Runtime.
    Command(Box<CommandRequest>),
    /// Immediate command-admission response.
    CommandResponse(CommandResponse),
    /// Event leaving Runtime.
    Event(EventMessage),
    /// Durable receiver acknowledgement used to clear the Runtime outbox.
    EventAck(EventAck),
    /// Lion asks the cloud composition to remove an archived thread's Runtime checkpoints.
    CheckpointPurge(CheckpointPurgeRequest),
    /// Agent-cloud reports the idempotent S3 purge result.
    CheckpointPurgeAck(CheckpointPurgeAck),
    /// Non-durable progress leaving Runtime.
    Observation(ObservationMessage),
    /// One dispatch failed outside the checkpointed Runtime state machine.
    ExecutionFailure(ExecutionFailure),
    /// Runtime request to the App Facade.
    AppFacadeRequest(AppFacadeRequest),
    /// App Facade response returned to Runtime.
    AppFacadeResponse(AppFacadeResponse),
}

/// Complete transport message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    /// Correlation and version metadata.
    pub metadata: Metadata,
    /// Typed payload.
    pub payload: Payload,
}

impl Envelope {
    /// Strictly decodes a JSON envelope and rejects fields unknown to the Rust DTO graph.
    ///
    /// Transport adapters should use this entry point rather than calling `serde_json` directly.
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` for malformed JSON, unknown fields, or failed envelope validation.
    pub fn decode_json(input: &str) -> Result<Self, ProtocolError> {
        let input_value: serde_json::Value =
            serde_json::from_str(input).map_err(|error| ProtocolError::InvalidJson {
                message: error.to_string(),
            })?;
        let mut deserializer = serde_json::Deserializer::from_str(input);
        let mut unknown = Vec::new();
        let envelope: Self = serde_ignored::deserialize(&mut deserializer, |path| {
            unknown.push(path.to_string());
        })
        .map_err(|error| ProtocolError::InvalidJson {
            message: error.to_string(),
        })?;
        deserializer
            .end()
            .map_err(|error| ProtocolError::InvalidJson {
                message: error.to_string(),
            })?;
        if let Some(path) = unknown.into_iter().next() {
            return Err(ProtocolError::UnknownField { path });
        }
        let canonical =
            serde_json::to_value(&envelope).map_err(|error| ProtocolError::InvalidJson {
                message: error.to_string(),
            })?;
        if let Some(path) = first_extra_field(&input_value, &canonical, "$") {
            return Err(ProtocolError::UnknownField { path });
        }
        envelope.validate()?;
        Ok(envelope)
    }

    /// Validates transport-level invariants before dispatch.
    ///
    /// # Errors
    ///
    /// Returns `ProtocolError` when the major version is unsupported or a required identifier is
    /// blank.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.metadata.version != CURRENT_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                actual: self.metadata.version,
            });
        }
        if self.metadata.message_id.trim().is_empty() {
            return Err(ProtocolError::MissingIdentifier("message_id"));
        }
        if self.metadata.correlation_id.trim().is_empty() {
            return Err(ProtocolError::MissingIdentifier("correlation_id"));
        }
        validate_dispatch(&self.metadata.dispatch)?;
        if let Some(context) = self.context() {
            validate_context(context)?;
        }
        if let Payload::Command(request) = &self.payload {
            validate_command_binding(request)?;
        }
        if let Payload::CheckpointPurge(request) = &self.payload {
            validate_purge_request(request)?;
        }
        if let Payload::CheckpointPurgeAck(ack) = &self.payload {
            validate_purge_ack(ack)?;
        }
        if let Payload::EventAck(ack) = &self.payload {
            validate_checkpoint_reference(&ack.checkpoint)?;
        }
        match &self.payload {
            Payload::Command(request) => {
                if let Some(reference) = &request.base_checkpoint {
                    validate_checkpoint_reference(reference)?;
                }
                if matches!(&request.command, AgentCommand::Cancel { .. })
                    && (request.base_checkpoint.is_some() || request.abandoned_run_id.is_some())
                {
                    return Err(ProtocolError::InvalidBinding(
                        "cancel must target live execution without checkpoint recovery metadata"
                            .to_owned(),
                    ));
                }
                if request.abandoned_run_id.is_some() && request.base_checkpoint.is_none() {
                    return Err(ProtocolError::InvalidBinding(
                        "an abandoned run requires a base checkpoint".to_owned(),
                    ));
                }
            }
            Payload::Event(message) => validate_checkpoint_reference(&message.checkpoint)?,
            _ => {}
        }
        Ok(())
    }

    fn context(&self) -> Option<&InvocationContext> {
        match &self.payload {
            Payload::Command(request) => match &request.command {
                AgentCommand::Enqueue { context, .. }
                | AgentCommand::SubmitInteraction { context, .. }
                | AgentCommand::Cancel { context } => Some(context),
            },
            Payload::Event(message) => Some(&message.event.context),
            Payload::Observation(message) => Some(&message.context),
            Payload::ExecutionFailure(message) => Some(&message.context),
            Payload::AppFacadeRequest(request) => match request {
                AppFacadeRequest::Invoke { context, .. }
                | AppFacadeRequest::Prepare { context, .. }
                | AppFacadeRequest::Commit { context, .. }
                | AppFacadeRequest::Reject { context, .. } => Some(context),
            },
            Payload::CommandResponse(response) => match response {
                CommandResponse::Accepted { context, .. }
                | CommandResponse::Rejected { context, .. } => Some(context),
            },
            Payload::EventAck(_)
            | Payload::CheckpointPurge(_)
            | Payload::CheckpointPurgeAck(_)
            | Payload::AppFacadeResponse(_) => None,
        }
    }
}

fn validate_purge_request(request: &CheckpointPurgeRequest) -> Result<(), ProtocolError> {
    if request.purge_id.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("purge_id"));
    }
    if request.conversation_key.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("conversation_key"));
    }
    if request.scope.scope_id.as_str().trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("scope_id"));
    }
    if request.thread_id.as_str().trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("thread_id"));
    }
    Ok(())
}

fn validate_purge_ack(ack: &CheckpointPurgeAck) -> Result<(), ProtocolError> {
    let (purge_id, conversation_key, thread_id) = match ack {
        CheckpointPurgeAck::Deleted {
            purge_id,
            conversation_key,
            thread_id,
            ..
        }
        | CheckpointPurgeAck::Failed {
            purge_id,
            conversation_key,
            thread_id,
            ..
        } => (purge_id, conversation_key, thread_id),
    };
    if purge_id.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("purge_id"));
    }
    if conversation_key.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("conversation_key"));
    }
    if thread_id.as_str().trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("thread_id"));
    }
    if let CheckpointPurgeAck::Failed { code, .. } = ack
        && code.trim().is_empty()
    {
        return Err(ProtocolError::MissingIdentifier("code"));
    }
    Ok(())
}

fn validate_dispatch(dispatch: &DispatchBinding) -> Result<(), ProtocolError> {
    if dispatch.dispatch_id.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("dispatch_id"));
    }
    if dispatch.agent_session_id.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("agent_session_id"));
    }
    Ok(())
}

fn validate_checkpoint_reference(reference: &CheckpointReference) -> Result<(), ProtocolError> {
    if reference.object_key.trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("checkpoint_object_key"));
    }
    if reference.format_version == 0
        || reference.sha256.len() != 64
        || !reference
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ProtocolError::InvalidBinding(
            "checkpoint reference is invalid".to_owned(),
        ));
    }
    Ok(())
}

fn validate_command_binding(request: &CommandRequest) -> Result<(), ProtocolError> {
    match (&request.command, &request.context_generation) {
        (AgentCommand::Enqueue { .. } | AgentCommand::SubmitInteraction { .. }, Some(value))
            if value.is_complete() => {}
        (AgentCommand::Enqueue { .. } | AgentCommand::SubmitInteraction { .. }, _) => {
            return Err(ProtocolError::InvalidBinding(
                "enqueue and submit_interaction require a context generation".to_owned(),
            ));
        }
        (AgentCommand::Cancel { .. }, None) => {}
        (AgentCommand::Cancel { .. }, Some(_)) => {
            return Err(ProtocolError::InvalidBinding(
                "cancel must not carry a context generation".to_owned(),
            ));
        }
    }
    match (&request.command, &request.llm_plan) {
        (AgentCommand::Enqueue { options, .. }, Some(plan)) => {
            validate_llm_plan(plan)?;
            if plan
                .route_for(&options.use_case, &options.model_mode)
                .is_none()
            {
                return Err(ProtocolError::InvalidBinding(
                    "LLM plan does not contain the command's primary selector".to_owned(),
                ));
            }
            Ok(())
        }
        (AgentCommand::Enqueue { .. }, None) => Err(ProtocolError::InvalidBinding(
            "enqueue requires a frozen LLM plan".to_owned(),
        )),
        (AgentCommand::SubmitInteraction { .. }, Some(plan)) => validate_llm_plan(plan),
        (AgentCommand::SubmitInteraction { .. }, None) => Err(ProtocolError::InvalidBinding(
            "submit_interaction requires the run's frozen LLM plan".to_owned(),
        )),
        (_, Some(_)) => Err(ProtocolError::InvalidBinding(
            "only commands that invoke the model may carry an LLM plan".to_owned(),
        )),
        (_, None) => Ok(()),
    }
}

fn validate_llm_plan(plan: &LlmExecutionPlan) -> Result<(), ProtocolError> {
    if plan.routes.is_empty() {
        return Err(ProtocolError::InvalidBinding(
            "LLM plan must contain at least one route".to_owned(),
        ));
    }
    let mut selectors = std::collections::HashSet::new();
    for binding in &plan.routes {
        let route = &binding.route;
        route
            .model_snapshot
            .generation_support
            .validate()
            .map_err(|error| ProtocolError::InvalidBinding(error.into()))?;
        llm_api::GenerationParameters {
            temperature: route.temperature,
            reasoning_effort: route.reasoning_effort.clone(),
            thinking: route.thinking,
            fast_mode: route.fast_mode,
        }
        .validate()
        .map_err(|error| ProtocolError::InvalidBinding(error.into()))?;
        if binding.use_case.0.trim().is_empty()
            || binding.model_mode.0.trim().is_empty()
            || route.backend.trim().is_empty()
            || route.profile.trim().is_empty()
            || route.model.trim().is_empty()
        {
            return Err(ProtocolError::InvalidBinding(
                "LLM use case, model mode, backend, profile, and model are required".to_owned(),
            ));
        }
        if !selectors.insert((binding.use_case.0.as_str(), binding.model_mode.0.as_str())) {
            return Err(ProtocolError::InvalidBinding(
                "LLM plan contains a duplicate selector".to_owned(),
            ));
        }
        if binding
            .revision
            .as_ref()
            .is_some_and(|value| value.trim().is_empty())
            || route
                .provider_preferences
                .iter()
                .any(|value| value.trim().is_empty())
            || route
                .reasoning_effort
                .as_ref()
                .is_some_and(|value| value.trim().is_empty())
            || route.temperature.is_some_and(|value| !value.is_finite())
            || route.context_window_tokens <= 20_000
        {
            return Err(ProtocolError::InvalidBinding(
                "LLM route contains invalid optional settings".to_owned(),
            ));
        }
    }
    Ok(())
}

fn first_extra_field(
    input: &serde_json::Value,
    canonical: &serde_json::Value,
    path: &str,
) -> Option<String> {
    match (input, canonical) {
        (serde_json::Value::Object(input), serde_json::Value::Object(canonical)) => {
            for (key, value) in input {
                let child_path = format!("{path}.{key}");
                let Some(expected) = canonical.get(key) else {
                    return Some(child_path);
                };
                if let Some(extra) = first_extra_field(value, expected, &child_path) {
                    return Some(extra);
                }
            }
            None
        }
        (serde_json::Value::Array(input), serde_json::Value::Array(canonical)) => input
            .iter()
            .zip(canonical)
            .enumerate()
            .find_map(|(index, (value, expected))| {
                first_extra_field(value, expected, &format!("{path}[{index}]"))
            }),
        _ => None,
    }
}

fn validate_context(context: &InvocationContext) -> Result<(), ProtocolError> {
    validate_scope(&context.scope)?;
    for (name, value) in [
        ("user_id", context.actor.user_id.as_str()),
        ("thread_id", context.thread_id.as_str()),
        ("run_id", context.run_id.as_str()),
        ("operation_id", context.operation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ProtocolError::MissingIdentifier(name));
        }
    }
    if context
        .actor
        .client_id
        .as_ref()
        .is_some_and(|client| client.as_str().trim().is_empty())
    {
        return Err(ProtocolError::MissingIdentifier("client_id"));
    }
    Ok(())
}

fn validate_scope(scope: &Scope) -> Result<(), ProtocolError> {
    if scope.scope_id.as_str().trim().is_empty() {
        return Err(ProtocolError::MissingIdentifier("scope_id"));
    }
    if scope
        .tenant_id
        .as_ref()
        .is_some_and(|tenant| tenant.as_str().trim().is_empty())
    {
        return Err(ProtocolError::MissingIdentifier("tenant_id"));
    }
    Ok(())
}

/// Invalid wire envelope.
#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum ProtocolError {
    /// JSON could not be decoded into an envelope.
    #[error("invalid protocol JSON: {message}")]
    InvalidJson {
        /// Safe parser diagnostic.
        message: String,
    },
    /// The payload contained a field outside the normative schema.
    #[error("unknown protocol field: {path}")]
    UnknownField {
        /// Serde path to the unexpected field.
        path: String,
    },
    /// The sender used a protocol major version this crate does not understand.
    #[error("unsupported protocol version {actual}")]
    UnsupportedVersion {
        /// Received major version.
        actual: u16,
    },
    /// A required identifier was blank.
    #[error("missing required identifier: {0}")]
    MissingIdentifier(&'static str),
    /// Environment binding is absent or disagrees with its Runtime command.
    #[error("invalid command binding: {0}")]
    InvalidBinding(String),
}

#[cfg(test)]
mod tests {
    use crate::execution::{
        AccessMode, Actor, AgentEvent, ClientId, ContentPart, EventId, Message, MessageRole,
        ModelMode, OperationId, RunId, RunOptions, Scope, ScopeId, TenantId, ThreadId, UseCase,
        UserId,
    };

    use super::*;

    #[test]
    fn rejects_unknown_protocol_version() {
        let envelope = Envelope {
            metadata: Metadata {
                version: CURRENT_VERSION + 1,
                message_id: "message-1".to_owned(),
                correlation_id: "correlation-1".to_owned(),
                causation_id: None,
                sent_at_unix_ms: 0,
                dispatch: dispatch(),
            },
            payload: Payload::Event(EventMessage {
                event: DurableEvent {
                    id: EventId::from("event-1"),
                    sequence: 1,
                    context: InvocationContext {
                        scope: Scope {
                            tenant_id: None,
                            scope_id: ScopeId::from("scope-1"),
                        },
                        actor: Actor {
                            user_id: UserId::from("user-1"),
                            client_id: Some(ClientId::from("client-1")),
                        },
                        surface_id: ConversationSurface::client_personal("user-1").unwrap(),
                        thread_id: ThreadId::from("thread-1"),
                        run_id: RunId::from("run-1"),
                        operation_id: OperationId::from("operation-1"),
                        deadline_unix_ms: None,
                        traceparent: None,
                    },
                    event: AgentEvent::Started,
                },
                checkpoint: checkpoint(),
            }),
        };

        assert_eq!(
            envelope.validate(),
            Err(ProtocolError::UnsupportedVersion {
                actual: CURRENT_VERSION + 1
            })
        );
    }

    #[test]
    fn command_fixture_round_trips_without_shape_drift() {
        let fixture = include_str!("../../../fixtures/execution/enqueue.v1.json");
        let envelope = Envelope::decode_json(fixture).expect("valid golden fixture");

        let expected = Envelope {
            metadata: Metadata {
                version: CURRENT_VERSION,
                message_id: "message-1".to_owned(),
                correlation_id: "correlation-1".to_owned(),
                causation_id: None,
                sent_at_unix_ms: 1_700_000_000_000,
                dispatch: dispatch(),
            },
            payload: Payload::Command(Box::new(CommandRequest {
                command: AgentCommand::Enqueue {
                    context: InvocationContext {
                        scope: Scope {
                            tenant_id: Some(TenantId::from("tenant-1")),
                            scope_id: ScopeId::from("scope-1"),
                        },
                        actor: Actor {
                            user_id: UserId::from("user-1"),
                            client_id: Some(ClientId::from("client-1")),
                        },
                        surface_id: ConversationSurface::client_personal("user-1").unwrap(),
                        thread_id: ThreadId::from("thread-1"),
                        run_id: RunId::from("run-1"),
                        operation_id: OperationId::from("operation-1"),
                        deadline_unix_ms: None,
                        traceparent: None,
                    },
                    message: Message {
                        continuation: None,
                        role: MessageRole::User,
                        content: vec![ContentPart::Text {
                            text: "hello".to_owned(),
                        }],
                    },
                    options: RunOptions {
                        use_case: UseCase("chat".to_owned()),
                        model_mode: ModelMode("auto".to_owned()),
                        access_mode: AccessMode::Interactive,
                        allow_tools: true,
                        max_steps: 8,
                        credit_budget: None,
                        debug: false,
                    },
                },
                context_generation: Some(ContextGeneration {
                    user_scope: "scope-1".to_owned(),
                    identity: "identity-1".to_owned(),
                    memory: "memory-1".to_owned(),
                    integration_guide: "guide-1".to_owned(),
                    installed_integrations: "installed-1".to_owned(),
                    scope_integrations: "scope-integrations-1".to_owned(),
                }),
                llm_plan: Some(LlmExecutionPlan {
                    routes: [
                        "chat",
                        "agent_info_merge",
                        "automation_judge",
                        "automation_diagnose",
                        "app_guide",
                        "workflow.action_operate",
                        "workflow.automation_generate",
                        "workflow.description_generate",
                    ]
                    .into_iter()
                    .map(|use_case| LlmRouteBinding {
                        use_case: UseCase(use_case.to_owned()),
                        model_mode: ModelMode("auto".to_owned()),
                        revision: Some("1700000000000".to_owned()),
                        route: LlmRoute {
                            backend: "openrouter".to_owned(),
                            profile: "cloud_openrouter".to_owned(),
                            model: "openai/gpt-5.4-mini".to_owned(),
                            provider_preferences: vec!["OpenAI".to_owned()],
                            temperature: Some(0.5),
                            cache_control: Some(true),
                            reasoning_effort: None,
                            thinking: None,
                            fast_mode: None,
                            context_window_tokens: 128_000,
                            model_snapshot: LlmModelSnapshot {
                                capabilities: llm_api::ModelCapabilities {
                                    input: vec!["image".to_owned()],
                                    tool_calling: true,
                                    structured_output: true,
                                },
                                generation_support: LlmGenerationSupport {
                                    temperature: Some(true),
                                    max_tokens: Some(true),
                                    thinking: Some(true),
                                    reasoning_efforts: Some(vec!["high".into()]),
                                    temperature_with_reasoning: Some(true),
                                    ..LlmGenerationSupport::default()
                                },
                            },
                        },
                    })
                    .collect(),
                }),
                base_checkpoint: None,
                abandoned_run_id: None,
            })),
        };
        assert_eq!(envelope, expected);
        assert_eq!(
            serde_json::to_value(envelope).expect("serializes"),
            serde_json::from_str::<serde_json::Value>(fixture).expect("fixture JSON")
        );
    }

    #[test]
    fn snapshot_is_required_and_support_metadata_is_strict() {
        let mut fixture: serde_json::Value = serde_json::from_str(ENQUEUE_FIXTURE).unwrap();
        fixture["payload"]["payload"]["llm_plan"]["routes"][0]["route"]
            .as_object_mut()
            .unwrap()
            .remove("model_snapshot");
        assert!(Envelope::decode_json(&fixture.to_string()).is_err());
        let mut envelope = Envelope::decode_json(ENQUEUE_FIXTURE).unwrap();
        let Payload::Command(command) = &mut envelope.payload else {
            panic!("command");
        };
        command.llm_plan.as_mut().unwrap().routes[0]
            .route
            .model_snapshot
            .generation_support
            .max_output_tokens = Some(0);
        assert!(envelope.validate().is_err());
    }

    #[test]
    fn generation_support_travels_in_catalog_spelling() {
        let support = LlmGenerationSupport {
            temperature: Some(true),
            max_tokens: Some(true),
            thinking: Some(true),
            reasoning_efforts: Some(vec!["low".into(), "high".into()]),
            reasoning_effort_default: Some("high".into()),
            temperature_with_reasoning: Some(false),
            temperature_max: Some(2.0),
            max_output_tokens: Some(8_192),
            fast_mode: Some(true),
        };
        let encoded = serde_json::to_value(&support).unwrap();
        assert_eq!(encoded["reasoningEfforts"], serde_json::json!(["low", "high"]));
        assert_eq!(encoded["temperatureWithReasoning"], serde_json::json!(false));
        assert_eq!(
            serde_json::from_value::<LlmGenerationSupport>(encoded).unwrap(),
            support
        );
        let ignored = serde_json::from_value::<LlmGenerationSupport>(serde_json::json!({
            "temperature": true, "maxTokens": true, "reasoningEfforts": ["low"],
            "temperatureWithReasoning": null, "temperatureMax": null, "maxOutputTokens": null,
            "reasoningIntensity": "high"
        }))
        .unwrap();
        assert_eq!(ignored.temperature, Some(true));
        assert_eq!(
            ignored.reasoning_efforts.as_deref(),
            Some(["low".to_string()].as_slice())
        );
    }

    fn dispatch() -> DispatchBinding {
        DispatchBinding {
            dispatch_id: "dispatch-1".to_owned(),
            agent_session_id: "agent-session-1".to_owned(),
        }
    }

    fn checkpoint() -> CheckpointReference {
        CheckpointReference {
            object_key: "checkpoints/abc.json".to_owned(),
            sha256: "a".repeat(64),
            format_version: 1,
        }
    }

    #[test]
    fn every_v1_golden_fixture_round_trips() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../schema/execution/envelope.v1.schema.json"
        ))
        .expect("valid JSON Schema document");
        let validator = jsonschema::validator_for(&schema).expect("valid JSON Schema semantics");
        for fixture in [
            include_str!("../../../fixtures/execution/enqueue.v1.json"),
            include_str!("../../../fixtures/execution/completed-event.v1.json"),
            include_str!("../../../fixtures/execution/interaction-required-event.v1.json"),
            include_str!("../../../fixtures/execution/failed-event.v1.json"),
            include_str!("../../../fixtures/execution/app-facade-prepare.v1.json"),
            include_str!("../../../fixtures/execution/observation.v1.json"),
            include_str!("../../../fixtures/execution/admission.v1.json"),
            include_str!("../../../fixtures/execution/app-facade-prepared.v1.json"),
            include_str!("../../../fixtures/execution/app-facade-user-action.v1.json"),
            include_str!("../../../fixtures/execution/event-ack.v1.json"),
            include_str!("../../../fixtures/execution/checkpoint-purge.v1.json"),
            include_str!("../../../fixtures/execution/checkpoint-purge-ack.v1.json"),
        ] {
            let json: serde_json::Value =
                serde_json::from_str(fixture).expect("valid golden fixture JSON");
            validator
                .validate(&json)
                .expect("golden fixture matches the normative schema");
            let envelope = Envelope::decode_json(fixture).expect("fixture matches Rust DTOs");
            assert_eq!(serde_json::to_value(envelope).expect("serializes"), json);
        }
    }

    #[test]
    fn rejects_blank_scoped_identifiers() {
        let fixture = include_str!("../../../fixtures/execution/enqueue.v1.json");
        let mut envelope: Envelope = serde_json::from_str(fixture).expect("valid golden fixture");
        let Payload::Command(request) = &mut envelope.payload else {
            panic!("enqueue fixture changed family");
        };
        let CommandRequest {
            command: AgentCommand::Enqueue { context, .. },
            ..
        } = request.as_mut()
        else {
            panic!("enqueue fixture changed family");
        };
        context.scope.scope_id = ScopeId::from(" ");
        assert_eq!(
            envelope.validate(),
            Err(ProtocolError::MissingIdentifier("scope_id"))
        );
    }

    #[test]
    fn strict_decoder_rejects_unknown_nested_fields() {
        let fixture = include_str!("../../../fixtures/execution/enqueue.v1.json");
        let input = fixture.replacen(
            "\"sent_at_unix_ms\": 1700000000000",
            "\"sent_at_unix_ms\": 1700000000000, \"unexpected\": true",
            1,
        );
        assert!(matches!(
            Envelope::decode_json(&input),
            Err(ProtocolError::UnknownField { .. })
        ));

        let mut input: serde_json::Value = serde_json::from_str(fixture).expect("fixture JSON");
        input["payload"]["payload"]["options"]["unexpected"] = serde_json::Value::Bool(true);
        assert!(matches!(
            Envelope::decode_json(&input.to_string()),
            Err(ProtocolError::UnknownField { .. })
        ));
    }

    #[test]
    fn abandoned_run_requires_a_base_checkpoint() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../../../fixtures/execution/enqueue.v1.json"))
                .unwrap();
        value["payload"]["payload"]["abandoned_run_id"] =
            serde_json::Value::String("run-abandoned".to_owned());

        assert!(matches!(
            Envelope::decode_json(&value.to_string()),
            Err(ProtocolError::InvalidBinding(_))
        ));
    }
}
