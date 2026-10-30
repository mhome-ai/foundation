//! Stable failures; callers never infer behavior from diagnostic text.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StorageErrorCode {
    InvalidInput,
    NotFound,
    Conflict,
    QuotaExceeded,
    VolumeFull,
    Integrity,
    Unauthorized,
    Unavailable,
    ScopeRemoving,
    SystemFolderProtected,
    RepositoryIdentityMismatch,
    ProtocolMismatch,
    EvictionFailed,
    Internal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MutationOutcome {
    NotApplied,
    Partial,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageError {
    pub code: StorageErrorCode,
    pub detail: String,
    pub outcome: MutationOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evicted_objects: Option<u64>,
}
impl StorageError {
    pub fn new(code: StorageErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            outcome: MutationOutcome::NotApplied,
            evicted_objects: None,
        }
    }
    pub fn uncertain(mut self) -> Self {
        self.outcome = MutationOutcome::Unknown;
        self
    }
    pub fn partial(mut self, count: u64) -> Self {
        self.outcome = MutationOutcome::Partial;
        self.evicted_objects = Some(count);
        self
    }
    pub fn http_status(&self) -> u16 {
        use StorageErrorCode::*;
        match self.code {
            InvalidInput | Integrity => 400,
            NotFound => 404,
            Unauthorized | SystemFolderProtected | ScopeRemoving => 403,
            Conflict | QuotaExceeded | RepositoryIdentityMismatch | EvictionFailed => 409,
            VolumeFull => 507,
            Unavailable | ProtocolMismatch => 503,
            Internal => 500,
        }
    }
}
impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.detail)
    }
}
impl std::error::Error for StorageError {}
