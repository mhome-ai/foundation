//! Closed catalog of public conversation run error codes.
//!
//! Host terminalize and Agent `Failed` both emit these strings. `status` and `failure.source`
//! are properties of the code. Unknown strings classify as `agent_failed` / `agent_runtime`.

use crate::execution::ExternalErrorKind;
use crate::{FailureSource, RunStatus};
use serde::{Deserialize, Serialize};

/// Public `runOutcome.errorCode` / Agent `Failed.code` catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RunErrorCode {
    Cancelled,
    RunInterrupted,
    ExecutorTerminated,
    ThreadExpired,
    ThreadArchived,
    AgentAdmissionTimeout,
    AgentDeadlineExceeded,
    AgentConnectionLost,
    AgentUnavailable,
    ProcessInterrupted,
    Protocol,
    InvalidEnvelope,
    InvalidFrame,
    InvalidJson,
    StaleDispatch,
    StaleSession,
    UnsupportedVersion,
    InvalidFamily,
    InvalidRequest,
    Unauthorized,
    Unavailable,
    RateLimited,
    ContextLimit,
    Timeout,
    Internal,
    OutputTruncated,
    CreditBudgetExceeded,
    ThreadBusy,
    AdmissionRejected,
    AgentExecutionFailed,
    InvalidModelFinish,
    InvalidModelToolCalls,
}

impl RunErrorCode {
    /// Catalog members in wire order. Tests treat this as the schema source of truth.
    pub const ALL: &[Self] = &[
        Self::Cancelled,
        Self::RunInterrupted,
        Self::ExecutorTerminated,
        Self::ThreadExpired,
        Self::ThreadArchived,
        Self::AgentAdmissionTimeout,
        Self::AgentDeadlineExceeded,
        Self::AgentConnectionLost,
        Self::AgentUnavailable,
        Self::ProcessInterrupted,
        Self::Protocol,
        Self::InvalidEnvelope,
        Self::InvalidFrame,
        Self::InvalidJson,
        Self::StaleDispatch,
        Self::StaleSession,
        Self::UnsupportedVersion,
        Self::InvalidFamily,
        Self::InvalidRequest,
        Self::Unauthorized,
        Self::Unavailable,
        Self::RateLimited,
        Self::ContextLimit,
        Self::Timeout,
        Self::Internal,
        Self::OutputTruncated,
        Self::CreditBudgetExceeded,
        Self::ThreadBusy,
        Self::AdmissionRejected,
        Self::AgentExecutionFailed,
        Self::InvalidModelFinish,
        Self::InvalidModelToolCalls,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "CANCELLED",
            Self::RunInterrupted => "RUN_INTERRUPTED",
            Self::ExecutorTerminated => "EXECUTOR_TERMINATED",
            Self::ThreadExpired => "THREAD_EXPIRED",
            Self::ThreadArchived => "THREAD_ARCHIVED",
            Self::AgentAdmissionTimeout => "AGENT_ADMISSION_TIMEOUT",
            Self::AgentDeadlineExceeded => "AGENT_DEADLINE_EXCEEDED",
            Self::AgentConnectionLost => "AGENT_CONNECTION_LOST",
            Self::AgentUnavailable => "AGENT_UNAVAILABLE",
            Self::ProcessInterrupted => "PROCESS_INTERRUPTED",
            Self::Protocol => "PROTOCOL",
            Self::InvalidEnvelope => "INVALID_ENVELOPE",
            Self::InvalidFrame => "INVALID_FRAME",
            Self::InvalidJson => "INVALID_JSON",
            Self::StaleDispatch => "STALE_DISPATCH",
            Self::StaleSession => "STALE_SESSION",
            Self::UnsupportedVersion => "UNSUPPORTED_VERSION",
            Self::InvalidFamily => "INVALID_FAMILY",
            Self::InvalidRequest => "INVALID_REQUEST",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Unavailable => "UNAVAILABLE",
            Self::RateLimited => "RATE_LIMITED",
            Self::ContextLimit => "CONTEXT_LIMIT",
            Self::Timeout => "TIMEOUT",
            Self::Internal => "INTERNAL",
            Self::OutputTruncated => "OUTPUT_TRUNCATED",
            Self::CreditBudgetExceeded => "CREDIT_BUDGET_EXCEEDED",
            Self::ThreadBusy => "THREAD_BUSY",
            Self::AdmissionRejected => "ADMISSION_REJECTED",
            Self::AgentExecutionFailed => "AGENT_EXECUTION_FAILED",
            Self::InvalidModelFinish => "INVALID_MODEL_FINISH",
            Self::InvalidModelToolCalls => "INVALID_MODEL_TOOL_CALLS",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|code| code.as_str() == raw)
    }

    pub const fn status(self) -> RunStatus {
        match self {
            Self::Cancelled
            | Self::RunInterrupted
            | Self::ExecutorTerminated
            | Self::ThreadExpired
            | Self::ThreadArchived => RunStatus::Interrupted,
            Self::AgentAdmissionTimeout
            | Self::AgentDeadlineExceeded
            | Self::AgentConnectionLost
            | Self::AgentUnavailable
            | Self::ProcessInterrupted
            | Self::Protocol
            | Self::InvalidEnvelope
            | Self::InvalidFrame
            | Self::InvalidJson
            | Self::StaleDispatch
            | Self::StaleSession
            | Self::UnsupportedVersion
            | Self::InvalidFamily => RunStatus::SystemFailed,
            Self::InvalidRequest
            | Self::Unauthorized
            | Self::Unavailable
            | Self::RateLimited
            | Self::ContextLimit
            | Self::Timeout
            | Self::Internal
            | Self::OutputTruncated
            | Self::CreditBudgetExceeded
            | Self::ThreadBusy
            | Self::AdmissionRejected
            | Self::AgentExecutionFailed
            | Self::InvalidModelFinish
            | Self::InvalidModelToolCalls => RunStatus::AgentFailed,
        }
    }

    /// `Some` only when the public outcome carries a `failure` object.
    pub const fn failure_source(self) -> Option<FailureSource> {
        match self.status() {
            RunStatus::AgentFailed => Some(match self {
                Self::InvalidRequest
                | Self::Unauthorized
                | Self::Unavailable
                | Self::RateLimited
                | Self::ContextLimit
                | Self::Timeout
                | Self::Internal
                | Self::OutputTruncated => FailureSource::LlmProvider,
                _ => FailureSource::AgentRuntime,
            }),
            _ => None,
        }
    }

    pub fn status_of(code: &str) -> RunStatus {
        Self::parse(code)
            .map(Self::status)
            .unwrap_or(RunStatus::AgentFailed)
    }

    pub fn failure_source_of(code: &str) -> Option<FailureSource> {
        Self::parse(code)
            .and_then(Self::failure_source)
            .or(if code.is_empty() {
                None
            } else {
                Some(FailureSource::AgentRuntime)
            })
    }
}

impl From<ExternalErrorKind> for RunErrorCode {
    fn from(kind: ExternalErrorKind) -> Self {
        match kind {
            ExternalErrorKind::InvalidRequest => Self::InvalidRequest,
            ExternalErrorKind::Unauthorized => Self::Unauthorized,
            ExternalErrorKind::Unavailable => Self::Unavailable,
            ExternalErrorKind::RateLimited => Self::RateLimited,
            ExternalErrorKind::ContextLimit => Self::ContextLimit,
            ExternalErrorKind::CreditBudgetExceeded => Self::CreditBudgetExceeded,
            ExternalErrorKind::Timeout => Self::Timeout,
            ExternalErrorKind::Cancelled => Self::Cancelled,
            ExternalErrorKind::Internal => Self::Internal,
        }
    }
}

impl AsRef<str> for RunErrorCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_strings_round_trip_and_are_unique() {
        let mut seen = std::collections::BTreeSet::new();
        for code in RunErrorCode::ALL {
            assert!(seen.insert(code.as_str()), "{}", code.as_str());
            assert_eq!(RunErrorCode::parse(code.as_str()), Some(*code));
            assert_eq!(
                serde_json::from_value::<RunErrorCode>(serde_json::json!(code.as_str())).unwrap(),
                *code
            );
        }
        assert_eq!(seen.len(), RunErrorCode::ALL.len());
        assert_eq!(RunErrorCode::parse("NOT_A_CATALOG_CODE"), None);
    }

    #[test]
    fn status_and_source_are_properties_of_the_code() {
        assert_eq!(RunErrorCode::Cancelled.status(), RunStatus::Interrupted);
        assert_eq!(RunErrorCode::Cancelled.failure_source(), None);
        assert_eq!(
            RunErrorCode::AgentDeadlineExceeded.status(),
            RunStatus::SystemFailed
        );
        assert_eq!(RunErrorCode::Unauthorized.status(), RunStatus::AgentFailed);
        assert_eq!(
            RunErrorCode::Unauthorized.failure_source(),
            Some(FailureSource::LlmProvider)
        );
        assert_eq!(
            RunErrorCode::CreditBudgetExceeded.failure_source(),
            Some(FailureSource::AgentRuntime)
        );
        assert_eq!(RunErrorCode::status_of("CANCELLED"), RunStatus::Interrupted);
        assert_eq!(
            RunErrorCode::status_of("THREAD_BUSY"),
            RunStatus::AgentFailed
        );
        assert_eq!(RunErrorCode::status_of("MADE_UP"), RunStatus::AgentFailed);
        assert_eq!(
            RunErrorCode::failure_source_of("MADE_UP"),
            Some(FailureSource::AgentRuntime)
        );
        assert_eq!(
            RunErrorCode::from(ExternalErrorKind::RateLimited),
            RunErrorCode::RateLimited
        );
    }
}
