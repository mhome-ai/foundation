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
    /// Private creation identity, used by SDKs to prevent renewal into a replacement Folder.
    pub folder_incarnation: String,
}

/// Host-owned native HTTPS origin, projected independently of Artifact routes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageEndpointProjection {
    pub base_url: String,
    pub host_identity: artifact_api::HostTlsIdentity,
}

/// Preserve the Storage failure through Core RPC and native Host adapters.
/// Only the typed SESSION_EXPIRED detail means that a data-plane retry is safe.
pub fn storage_error_from_core(
    code: &str,
    detail: impl Into<String>,
    details: Option<serde_json::Value>,
) -> StorageError {
    if let Some(error) =
        details.and_then(|value| serde_json::from_value::<StorageError>(value).ok())
    {
        return error;
    }
    use StorageErrorCode as S;
    let code = match code {
        "BAD_REQUEST" => S::InvalidInput,
        "NOT_FOUND" => S::NotFound,
        "UNAUTHORIZED" | "UNAUTHENTICATED" => S::Unauthorized,
        "PRECONDITION_FAIL" => S::Conflict,
        "INTERNAL" => S::Internal,
        _ => S::Unavailable,
    };
    let error = StorageError::new(code, detail);
    if matches!(code, S::Internal | S::Unavailable) {
        error.uncertain()
    } else {
        error
    }
}

#[cfg(test)]
mod storage_transport_tests {
    use super::*;
    #[test]
    fn expiry_survives_core_rpc_without_reclassifying_binding_revocation() {
        let expired = StorageError::new(StorageErrorCode::SessionExpired, "expired");
        let decoded = storage_error_from_core(
            "UNAUTHENTICATED",
            "Core exception",
            Some(serde_json::to_value(&expired).unwrap()),
        );
        assert_eq!(decoded, expired);
        assert_eq!(decoded.http_status(), 401);
        assert_eq!(
            storage_error_from_core("UNAUTHENTICATED", "binding revoked", None).http_status(),
            403
        );
        for error in [
            StorageError::new(StorageErrorCode::Unavailable, "offline").partial(2),
            StorageError::new(StorageErrorCode::Unavailable, "lost response").uncertain(),
        ] {
            assert_eq!(
                storage_error_from_core(
                    "TEMPORARY_UNAVAILABLE",
                    "Core exception",
                    Some(serde_json::to_value(&error).unwrap())
                ),
                error
            );
        }
    }
}
