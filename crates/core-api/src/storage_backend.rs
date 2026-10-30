//! Core/Plugin byte transport. Logical Folder policy remains in Core.
use serde::{Deserialize, Serialize};
pub const STORAGE_BACKEND_PROTOCOL_VERSION: &str = "storage.backend.v1";
pub const STORAGE_BACKEND_DESCRIBE: &str = "/storage/backend/describe";
pub const STORAGE_BACKEND_GRANT: &str = "/storage/backend/grant";
pub const STORAGE_BACKEND_PREPARE_WRITE: &str = "/storage/backend/prepare-write";
pub const STORAGE_GATEWAY_PREFIX: &str = "/storage/v4/";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageBackendPermission {
    Read,
    Write,
    Stat,
    Delete,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageBackendGrantRequest {
    pub repository_id: String,
    pub owner: String,
    pub object_id: String,
    pub permission: StorageBackendPermission,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageBackendPrepareWriteRequest {
    pub repository_id: String,
    pub owner: String,
    pub object: storage_api::backend::Blob,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBackendDescription {
    pub protocol_version: String,
    pub capacity: storage_api::backend::Capacity,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum StorageBackendReply<T> {
    Ok { value: T },
    Error { error: storage_api::StorageError },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageBackendGrant {
    pub repository_id: String,
    pub url: String,
    pub token: String,
    pub host_identity: artifact_api::HostTlsIdentity,
    pub expires_at_ms: u64,
}

/// Native gateway transport is local to the hosting process. Never return this descriptor to App callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageGatewayDescriptor {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageGatewayRequest {
    pub grant: String,
    pub access_token: String,
}
