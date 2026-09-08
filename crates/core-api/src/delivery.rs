//! Core -> Host delivery contract. Host executes a resolved destination; it never resolves
//! recipients or chooses fallback routes. Completion describes evidence, not user receipt.
use crate::MwsMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HostDeliveryRequest {
    /// Stable across retransmission of the same attempt. A new attempt needs a new ID.
    pub delivery_id: String,
    pub expires_at_unix_ms: i64,
    pub destination: HostDeliveryDestination,
}

impl HostDeliveryRequest {
    pub fn remaining(&self) -> std::time::Duration {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(u128::MAX);
        let remaining = (self.expires_at_unix_ms.max(0) as u128).saturating_sub(now);
        std::time::Duration::from_millis(remaining.min(u64::MAX as u128) as u64)
    }
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(u128::MAX);
        self.expires_at_unix_ms <= 0 || now >= self.expires_at_unix_ms as u128
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostDeliveryDestination {
    AppConnection {
        client_id: String,
        message: MwsMessage,
    },
    NodeConnection {
        connection_key: String,
        message: MwsMessage,
    },
    Messaging {
        request: crate::MessagingDeliveryRequest,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum DeliveryOutcome {
    Accepted { boundary: DeliveryAcceptance },
    Failed { code: String, message: String },
    Unknown { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeliveryAcceptance {
    /// The authenticated connection's writer accepted the frame, not the remote application.
    ConnectionQueue,
    /// The provider API accepted the operation, not a user-read acknowledgement.
    Provider,
}

impl DeliveryOutcome {
    pub fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }

    pub fn connection(accepted: bool) -> Self {
        if accepted {
            Self::Accepted {
                boundary: DeliveryAcceptance::ConnectionQueue,
            }
        } else {
            Self::Failed {
                code: "UNREACHABLE".into(),
                message: "Connection did not accept delivery".into(),
            }
        }
    }
}

/// Connection fencing is control, not delivery. The key must identify an exact session,
/// so delayed control requests cannot close a replacement connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum HostConnectionControl {
    DisconnectNode { connection_key: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acceptance_is_not_a_boolean_receipt() {
        let result = DeliveryOutcome::connection(true);
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            serde_json::json!({
                "status": "accepted", "boundary": "connectionQueue"
            })
        );
        assert!(
            serde_json::from_value::<DeliveryOutcome>(serde_json::json!({"delivered": true}))
                .is_err()
        );
    }

    #[test]
    fn obsolete_output_effects_are_rejected() {
        assert!(serde_json::from_value::<crate::ServiceCoreOutput>(
            serde_json::json!({"effects": []})
        )
        .is_err());
    }

    #[test]
    fn unknown_is_not_failure_or_acceptance() {
        let result = DeliveryOutcome::Unknown {
            message: "Timed out after dispatch".into(),
        };
        assert!(!result.is_accepted());
        let encoded = serde_json::to_value(&result).unwrap();
        assert_eq!(
            serde_json::from_value::<DeliveryOutcome>(encoded).unwrap(),
            result
        );
    }
}
