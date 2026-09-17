//! Stable error semantics exposed by external ports.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Adapter-independent category used by Runtime policy and retry decisions.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalErrorKind {
    /// The request violates a contract or provider constraint.
    InvalidRequest,
    /// The caller or configured credential is not authorized.
    Unauthorized,
    /// The dependency is temporarily unavailable.
    Unavailable,
    /// The dependency rejected the request due to a rate limit.
    RateLimited,
    /// The selected model cannot accept the supplied context.
    ContextLimit,
    /// The run consumed more credits than its frozen admission budget.
    CreditBudgetExceeded,
    /// The operation exceeded its deadline.
    Timeout,
    /// The operation was cancelled by its owner.
    Cancelled,
    /// The adapter failed without a more specific stable classification.
    Internal,
}

/// Sanitized error returned by LLM, App Facade, and event ports.
#[derive(Clone, Debug, Error, Eq, PartialEq, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct ExternalError {
    /// Stable machine-readable category.
    pub kind: ExternalErrorKind,
    /// Safe diagnostic message without credentials or infrastructure internals.
    pub message: String,
    /// Adapter-supplied retry delay when retrying is meaningful.
    pub retry_after_ms: Option<u64>,
}
