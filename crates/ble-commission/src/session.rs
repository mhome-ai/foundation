//! Shared state of one pod or Host session, and its snapshots.
use app_facade_api::host_provision::{HostDeviceInfo, ProvisionSession, ProvisionState};
use app_facade_api::pod::{
    AuthorizationPrompt, CommissionError, CommissionErrorCode, CommissionSession, CommissionState,
    DeviceKind, PodCandidate, PodDeviceInfo, SessionMode, WifiNetwork,
};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, watch};
use tracing::warn;

use crate::platform::CloudEndpoints;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Connecting,
    AwaitingCode,
    Securing,
    ReadingInfo,
    AwaitingWifi,
    JoiningWifi,
    AwaitingAuthorization,
    Issuing,
    Delivering,
    Activating,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl Phase {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut
        )
    }

    /// From delivery on, Lion decides the outcome; neither the owner lease nor
    /// the session lifetime ends the session.
    pub fn runs_to_conclusion(self) -> bool {
        matches!(self, Self::Delivering | Self::Activating)
    }

    /// The link is idle and must stay connected.
    pub fn waits_for_user(self) -> bool {
        matches!(
            self,
            Self::AwaitingCode | Self::AwaitingWifi | Self::AwaitingAuthorization
        )
    }

    fn commission_state(self) -> CommissionState {
        match self {
            Self::Connecting => CommissionState::Connecting,
            Self::AwaitingCode => CommissionState::AwaitingCode,
            Self::Securing => CommissionState::Securing,
            Self::ReadingInfo => CommissionState::ReadingInfo,
            Self::AwaitingWifi => CommissionState::AwaitingWifi,
            Self::JoiningWifi => CommissionState::JoiningWifi,
            Self::AwaitingAuthorization => CommissionState::AwaitingAuthorization,
            Self::Issuing => CommissionState::Issuing,
            Self::Delivering => CommissionState::Delivering,
            Self::Activating => CommissionState::Activating,
            Self::Completed => CommissionState::Completed,
            Self::Failed => CommissionState::Failed,
            Self::Cancelled => CommissionState::Cancelled,
            Self::TimedOut => CommissionState::TimedOut,
        }
    }

    fn provision_state(self) -> ProvisionState {
        match self {
            Self::Connecting => ProvisionState::Connecting,
            Self::AwaitingCode => ProvisionState::AwaitingCode,
            Self::Securing => ProvisionState::Securing,
            Self::ReadingInfo => ProvisionState::ReadingInfo,
            Self::AwaitingWifi => ProvisionState::AwaitingWifi,
            Self::JoiningWifi => ProvisionState::JoiningWifi,
            Self::Completed => ProvisionState::Completed,
            Self::Failed => ProvisionState::Failed,
            Self::Cancelled => ProvisionState::Cancelled,
            Self::TimedOut => ProvisionState::TimedOut,
            Self::AwaitingAuthorization | Self::Issuing | Self::Delivering | Self::Activating => {
                unreachable!("Host sessions have no credential phases")
            }
        }
    }
}

pub(crate) enum Op {
    Code(String),
    Wifi {
        ssid: String,
        password: String,
    },
    /// `join` waits for the scan that was already running when it was asked.
    Scan {
        join: bool,
    },
    Authorize {
        scope_id: String,
        endpoints: CloudEndpoints,
    },
}

pub(crate) struct Command {
    pub op: Op,
    pub done: oneshot::Sender<()>,
}

pub(crate) struct Live {
    pub kind: DeviceKind,
    pub id: String,
    pub revision: u64,
    pub phase: Phase,
    pub candidate: PodCandidate,
    pub expires_at_ms: i64,
    pub lease_expires_ms: i64,
    pub networks: Vec<WifiNetwork>,
    pub error: Option<CommissionError>,
    pub device: Option<PodDeviceInfo>,
    pub authorization: Option<AuthorizationPrompt>,
    pub pod_id: Option<String>,
    pub mode: Option<SessionMode>,
    pub host: Option<HostDeviceInfo>,
    pub addresses: Vec<String>,
    pub claim_token: Option<String>,
    /// A code, Wi-Fi or authorization request is queued or running.
    pub busy: bool,
    /// A Wi-Fi scan is running.
    pub scanning: bool,
    /// The session's link is closed and the radio is free again.
    pub released: bool,
    pub commands: mpsc::UnboundedSender<Command>,
    pub cancel: watch::Sender<bool>,
}

impl Live {
    pub fn set(&mut self, phase: Phase, error: Option<CommissionError>) {
        self.phase = phase;
        self.error = error;
    }

    pub fn pod_snapshot(&self) -> CommissionSession {
        CommissionSession {
            session_id: self.id.clone(),
            revision: self.revision,
            state: self.phase.commission_state(),
            candidate: self.candidate.clone(),
            expires_at_ms: self.expires_at_ms,
            device: self.device.clone(),
            networks: self.networks.clone(),
            authorization: self.authorization.clone(),
            pod_id: self.pod_id.clone(),
            mode: self.mode,
            error: self.error.clone(),
        }
    }

    pub fn host_snapshot(&self) -> ProvisionSession {
        ProvisionSession {
            session_id: self.id.clone(),
            revision: self.revision,
            state: self.phase.provision_state(),
            candidate: self.candidate.clone(),
            expires_at_ms: self.expires_at_ms,
            host: self.host.clone(),
            networks: self.networks.clone(),
            addresses: self.addresses.clone(),
            claim_token: self.claim_token.clone(),
            error: self.error.clone(),
        }
    }

    pub fn snapshot(&self) -> Value {
        let (value, problem) = match self.kind {
            DeviceKind::Pod => {
                let snapshot = self.pod_snapshot();
                (serde_json::to_value(&snapshot), snapshot.validate().err())
            }
            DeviceKind::Host => {
                let snapshot = self.host_snapshot();
                (serde_json::to_value(&snapshot), snapshot.validate().err())
            }
        };
        if let Some(problem) = problem {
            warn!(problem, phase = ?self.phase, "session snapshot is invalid");
        }
        value.unwrap_or(Value::Null)
    }
}

pub(crate) fn error(
    code: CommissionErrorCode,
    message: &str,
    detail: Option<String>,
) -> CommissionError {
    CommissionError {
        code,
        message: message.to_string(),
        detail,
    }
}

pub(crate) fn noun(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Pod => "pod",
        DeviceKind::Host => "Host",
    }
}

pub(crate) fn disconnected(kind: DeviceKind, detail: impl ToString) -> CommissionError {
    error(
        CommissionErrorCode::Disconnected,
        &format!("The connection to the {} was lost.", noun(kind)),
        Some(detail.to_string()),
    )
}
