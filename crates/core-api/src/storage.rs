//! Core-owned Storage access and transport. Domain types have one source in storage-api.
use serde::{Deserialize, Serialize};
pub use storage_api::*;
pub const STORAGE_NODE_TYPE: &str = "storage";
pub const STORAGE_PROTOCOL_VERSION: &str = storage_api::PROTOCOL_VERSION;
pub const STORAGE_READINESS_ROUTE: &str = "/storage/readiness";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectStorageSession {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_identity: Option<artifact_api::HostTlsIdentity>,
    pub protocol_version: String,
    pub folder: ObjectStorageFolder,
    pub endpoint: String,
    pub access_token: String,
    #[serde(with = "storage_api::decimal_u64")]
    pub expires_at_ms: u64,
    pub permissions: Vec<StoragePermission>,
}

/// Host-owned native HTTPS origin, projected independently of Artifact routes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageEndpointProjection {
    pub base_url: String,
    pub host_identity: artifact_api::HostTlsIdentity,
}
