use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

use crate::StorageWriteState;

pub mod decimal_u64 {
    use super::*;

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        parse(&value).map_err(serde::de::Error::custom)
    }

    pub(super) fn parse(value: &str) -> Result<u64, &'static str> {
        if value == "0"
            || (value.starts_with(|ch: char| matches!(ch, '1'..='9'))
                && value.bytes().all(|byte| byte.is_ascii_digit()))
        {
            value
                .parse()
                .map_err(|_| "must be an unsigned 64-bit decimal string")
        } else {
            Err("must be a canonical unsigned decimal string")
        }
    }
}

/// Nullable byte limits use the same lossless wire representation as other management counts.
pub mod optional_decimal_u64 {
    use super::*;
    pub fn serialize<S: Serializer>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
        match value {
            Some(value) => decimal_u64::serialize(value, serializer),
            None => serializer.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<u64>, D::Error> {
        Option::<String>::deserialize(deserializer)?
            .map(|value| decimal_u64::parse(&value).map_err(serde::de::Error::custom))
            .transpose()
    }
}

/// Set or clear the shared subtree limit. Last successfully committed setting wins.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectStorageSetFolderLimitRequest {
    pub folder_id: String,
    #[serde(with = "optional_decimal_u64")]
    pub max_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageFolderQuota {
    #[serde(with = "optional_decimal_u64")]
    pub max_bytes: Option<u64>,
    #[serde(with = "decimal_u64")]
    pub used_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub reserved_bytes: u64,
    /// Minimum remaining allowance across this Folder and every limited ancestor.
    #[serde(with = "optional_decimal_u64")]
    pub remaining_bytes: Option<u64>,
    pub exceeded: bool,
}

pub fn validate_folder_max_bytes(value: Option<u64>) -> Result<(), crate::StorageError> {
    if value.is_some_and(|value| value == 0 || value > i64::MAX as u64) {
        return Err(crate::StorageError::new(
            crate::StorageErrorCode::InvalidInput,
            "maxBytes must be null or a positive supported decimal byte count",
        ));
    }
    Ok(())
}

/// Idempotent creation of an immutable logical path in the caller's namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectStorageCreateFolderRequest {
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectStorageDeleteFolderRequest {
    pub folder_id: String,
    #[serde(default)]
    pub recursive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageDeleteFolderResponse {
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectStorageFolderIdRequest {
    pub folder_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ObjectStorageSessionRequest {
    pub folder_id: String,
    /// SDK renewal fence; never a user-selected Folder ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_incarnation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageFolder {
    pub quota: ObjectStorageFolderQuota,
    /// Immutable, case-sensitive logical path; never a filesystem path.
    pub id: String,
    pub system: bool,
    /// Cleanup was requested and must finish before this path can be reused.
    pub deleting: bool,
    #[serde(with = "decimal_u64")]
    pub used_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub reserved_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub object_count: u64,
    #[serde(with = "decimal_u64")]
    pub created_at_ms: u64,
    #[serde(with = "decimal_u64")]
    pub updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageFolderList {
    pub folders: Vec<ObjectStorageFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageFilesystemCapacity {
    #[serde(with = "decimal_u64")]
    pub total_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageUsage {
    #[serde(with = "decimal_u64")]
    pub used_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub reserved_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub writable_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageSafety {
    #[serde(with = "decimal_u64")]
    pub warning_at_available_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub stop_at_available_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageOverview {
    pub backends: Vec<ObjectStorageBackend>,
    pub protocol_version: String,
    pub write_state: StorageWriteState,
    pub write_available: bool,
    pub filesystem: ObjectStorageFilesystemCapacity,
    pub storage: ObjectStorageUsage,
    pub safety: ObjectStorageSafety,
    #[serde(with = "decimal_u64")]
    pub folder_count: u64,
    #[serde(with = "decimal_u64")]
    pub object_count: u64,
    #[serde(with = "decimal_u64")]
    pub current_space_folder_count: u64,
    #[serde(with = "decimal_u64")]
    pub current_space_object_count: u64,
    #[serde(with = "decimal_u64")]
    pub current_space_used_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub current_space_reserved_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub other_spaces_used_bytes: u64,
    #[serde(with = "decimal_u64")]
    pub active_upload_count: u64,
    #[serde(with = "decimal_u64")]
    pub pending_deletion_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageBackend {
    pub kind: String,
    pub repository_id: String,
    pub online: bool,
    pub capacity: Option<ObjectStorageFilesystemCapacity>,
}
