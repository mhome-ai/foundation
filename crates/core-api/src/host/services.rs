//! Latest process/service management state. Independent of any Space.
use serde::{Deserialize, Serialize};

/// An in-memory latest-state subscription, not a queued business event.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaitServiceRuntimeStateRequest {
    pub process_generation: Option<String>,
    pub revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceRuntimeState {
    pub service_id: String,
    pub process_generation: String,
    pub process_id: u32,
    pub revision: u64,
    pub phase: ServiceRuntimePhase,
    pub reason: Option<String>,
    pub permissions: crate::host::permissions::ServicePermissionState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServiceRuntimePhase {
    WaitingForPermissions,
    Starting,
    Ready,
    Stopping,
    Failed,
    Stopped,
}

impl ServiceRuntimePhase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WaitingForPermissions => "waitingForPermissions",
            Self::Starting => "starting",
            Self::Ready => "ready",
            Self::Stopping => "stopping",
            Self::Failed => "failed",
            Self::Stopped => "stopped",
        }
    }
}
