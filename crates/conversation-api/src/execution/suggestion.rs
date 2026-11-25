//! Trusted, non-conversation execution for Suggestion generation.
//!
//! These values are assigned by Core/Lion admission, never supplied by a model or
//! decoded from the public generate input. A scheduled service is not an enrolled
//! Space Client. Payment attribution does not grant business authority.

use serde::{Deserialize, Serialize};

use super::{OperationId, RunId, SpaceContext, UserId};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum SuggestionTrigger {
    #[serde(deserialize_with = "crate::execution::deserialize_empty_variant")]
    Scheduled,
    Explicit {
        #[serde(rename = "userId")]
        user_id: UserId,
    },
}

/// A reference resolved by the deployment's existing billing adapter. The owner
/// is not copied into the execution identity or retained as a permanent payer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SuggestionPayer {
    User(UserId),
    CurrentSpaceOwner(SpaceContext),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionInvocationContext {
    pub space: SpaceContext,
    pub run_id: RunId,
    pub operation_id: OperationId,
    pub trigger: SuggestionTrigger,
    pub deadline_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

impl SuggestionInvocationContext {
    /// Shape validation is not authentication, Space access, or an enabled-job
    /// check. The deployment must also bind the run to the backend currently
    /// responsible for this Space (Core or Lion).
    pub fn validate(&self) -> Result<(), &'static str> {
        if !valid_id(self.space.space_id.as_str())
            || !self
                .space
                .tenant_id
                .as_ref()
                .is_some_and(|id| valid_id(id.as_str()))
            || !valid_id(self.run_id.as_str())
            || !valid_id(self.operation_id.as_str())
            || self.deadline_unix_ms == 0
            || self.deadline_unix_ms > 9_007_199_254_740_991
        {
            return Err("Suggestion execution requires a Space, run, operation and deadline");
        }
        if let SuggestionTrigger::Explicit { user_id } = &self.trigger
            && !valid_id(user_id.as_str())
        {
            return Err("Explicit Suggestion generation requires the verified requesting user");
        }
        Ok(())
    }

    pub fn payer(&self) -> Result<SuggestionPayer, &'static str> {
        self.validate()?;
        Ok(match &self.trigger {
            SuggestionTrigger::Explicit { user_id } => SuggestionPayer::User(user_id.clone()),
            SuggestionTrigger::Scheduled => SuggestionPayer::CurrentSpaceOwner(self.space.clone()),
        })
    }
}

/// Minimal invocation metadata needed by shared model/tool workflows. Concrete
/// contexts retain their distinct authority and transport contracts.
pub trait ExecutionContext: Clone + Send + Sync {
    fn space(&self) -> &SpaceContext;
    fn run_id(&self) -> &RunId;
    fn operation_id(&self) -> &OperationId;
    fn with_operation_id(&self, operation_id: OperationId) -> Self;
}

impl ExecutionContext for SuggestionInvocationContext {
    fn space(&self) -> &SpaceContext {
        &self.space
    }
    fn run_id(&self) -> &RunId {
        &self.run_id
    }
    fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }
    fn with_operation_id(&self, operation_id: OperationId) -> Self {
        Self {
            operation_id,
            ..self.clone()
        }
    }
}

impl ExecutionContext for super::InvocationContext {
    fn space(&self) -> &SpaceContext {
        &self.space
    }
    fn run_id(&self) -> &RunId {
        &self.run_id
    }
    fn operation_id(&self) -> &OperationId {
        &self.operation_id
    }
    fn with_operation_id(&self, operation_id: OperationId) -> Self {
        Self {
            operation_id,
            ..self.clone()
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.chars().count() <= 256
        && value.trim() == value
        && !value.chars().any(char::is_control)
}
