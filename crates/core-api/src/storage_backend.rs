//! Byte-backend control protocol. Folder policy and the placement catalog belong to Core.
use serde::{Deserialize, Serialize};
pub const STORAGE_BACKEND_DESCRIBE: &str = "/storage/backend/describe";
pub const STORAGE_BACKEND_GRANT: &str = "/storage/backend/grant";
pub const STORAGE_GATEWAY_PREFIX: &str = "/storage/v3/";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageBackendGrantRequest {
    pub repository_id: String,
    pub owner: String,
    pub object_id: String,
    pub permission: String,
    pub size_bytes: Option<u64>,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
