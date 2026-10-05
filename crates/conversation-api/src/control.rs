//! Authoritative owner control state. History and live cursors never order this projection.
use crate::{ActiveRun, ConversationQueue, ConversationSurface, PendingInteraction, ThreadSummary};
use serde::{Deserialize, Serialize};

/// Fixed-width decimal pair (catalog epoch, state revision), lexicographically ordered on every client.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct ControlVersion(String);
impl ControlVersion {
    pub fn new(catalog: u64, state: u64) -> Self {
        Self(format!("{catalog:020}:{state:020}"))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl<'de> Deserialize<'de> for ControlVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        let bytes = value.as_bytes();
        if bytes.len() != 41
            || bytes[20] != b':'
            || !bytes[..20]
                .iter()
                .chain(&bytes[21..])
                .all(u8::is_ascii_digit)
            || value[..20].parse::<u64>().is_err()
            || value[21..].parse::<u64>().is_err()
        {
            return Err(serde::de::Error::custom("invalid control version"));
        }
        Ok(Self(value))
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ControlGetRequest {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConversationControl {
    pub surface_id: ConversationSurface,
    pub version: ControlVersion,
    pub active_thread: Option<ThreadSummary>,
    pub active_run: Option<ActiveRun>,
    pub pending_interaction: Option<PendingInteraction>,
    pub queue: ConversationQueue,
    /// History cursor for the active thread. Fetch history when behind; do not use it to order control.
    pub history_version: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlledResponse<T> {
    pub receipt: T,
    /// Null means the command committed but its projection could not be read; refetch control.
    pub control: Option<ConversationControl>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlUpdatedEventType {
    #[default]
    #[serde(rename = "control.updated")]
    ControlUpdated,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlUpdatedEvent {
    #[serde(rename = "type")]
    pub event_type: ControlUpdatedEventType,
    pub surface_id: ConversationSurface,
    pub data: ConversationControl,
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalog_epoch_dominates_thread_revision() {
        assert!(ControlVersion::new(2, 0) > ControlVersion::new(1, u64::MAX));
        assert!(ControlVersion::new(2, 10) > ControlVersion::new(2, 9));
    }
    #[test]
    fn invalid_versions_never_enter_ordering() {
        for value in [
            "1:2",
            "00000000000000000000:18446744073709551616",
            "0000000000000000000é:00000000000000000000",
        ] {
            assert!(serde_json::from_value::<ControlVersion>(serde_json::json!(value)).is_err());
        }
        let max = ControlVersion::new(u64::MAX, u64::MAX);
        assert_eq!(
            serde_json::from_str::<ControlVersion>(&serde_json::to_string(&max).unwrap()).unwrap(),
            max
        );
    }
}
