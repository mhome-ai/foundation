use serde::{Deserialize, Serialize};
use std::fmt;

pub const STORAGE_NODE_TYPE: &str = "storage";
pub const STORAGE_PROTOCOL_VERSION: &str = "storage.v1";

pub const STORAGE_DESCRIBE_ROUTE: &str = "/storage/describe";
pub const STORAGE_ENSURE_NAMESPACE_ROUTE: &str = "/storage/namespace/ensure";
pub const STORAGE_NAMESPACE_STATUS_ROUTE: &str = "/storage/namespace/status";
pub const STORAGE_UPDATE_NAMESPACE_POLICY_ROUTE: &str = "/storage/namespace/update-policy";
pub const STORAGE_LIST_NAMESPACES_ROUTE: &str = "/storage/namespaces/list";
pub const STORAGE_ISSUE_SESSION_ROUTE: &str = "/storage/session/issue";
pub const STORAGE_STATS_ROUTE: &str = "/storage/stats";
pub const STORAGE_READINESS_ROUTE: &str = "/storage/readiness";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageOverflowStrategy {
    #[default]
    Reject,
    EvictOldest,
}

impl StorageOverflowStrategy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reject => "reject",
            Self::EvictOldest => "evictOldest",
        }
    }

    pub fn parse(value: &str) -> Result<Self, UnsupportedStorageOverflowStrategy> {
        match value {
            "reject" => Ok(Self::Reject),
            "evictOldest" => Ok(Self::EvictOldest),
            _ => Err(UnsupportedStorageOverflowStrategy(value.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedStorageOverflowStrategy(String);

impl fmt::Display for UnsupportedStorageOverflowStrategy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "unsupported storage overflow strategy: {}",
            self.0
        )
    }
}

impl std::error::Error for UnsupportedStorageOverflowStrategy {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StoragePermission {
    Read,
    Write,
    Delete,
    List,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageEnsureNamespaceRequest {
    pub name: String,
    pub created_by_node_type: String,
    pub created_by_node_id: String,
    pub max_bytes: u64,
    #[serde(default)]
    pub overflow_strategy: StorageOverflowStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageNamespacePolicyUpdate {
    pub namespace_id: String,
    pub expected_revision: u64,
    pub max_bytes: u64,
    pub overflow_strategy: StorageOverflowStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageNamespaceIdRequest {
    pub namespace_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageSessionRequest {
    pub namespace_id: String,
    #[serde(default)]
    pub permissions: Vec<StoragePermission>,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageNamespacePolicy {
    pub max_bytes: u64,
    pub overflow_strategy: StorageOverflowStrategy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageNamespace {
    pub id: String,
    pub name: String,
    pub created_by_node_type: String,
    pub created_by_node_id: String,
    pub max_bytes: u64,
    pub overflow_strategy: StorageOverflowStrategy,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub object_count: u64,
    pub evicted_object_count: u64,
    pub evicted_bytes: u64,
    pub rejected_write_count: u64,
    pub revision: u64,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
}

impl StorageNamespace {
    pub const fn policy(&self) -> StorageNamespacePolicy {
        StorageNamespacePolicy {
            max_bytes: self.max_bytes,
            overflow_strategy: self.overflow_strategy,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageNamespaceList {
    pub namespaces: Vec<StorageNamespace>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSession {
    pub protocol_version: String,
    pub repository_id: String,
    pub namespace: StorageNamespace,
    pub endpoint: String,
    pub access_token: String,
    pub expires_at_ms: u64,
    pub permissions: Vec<StoragePermission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageAvailability {
    pub available: bool,
    pub protocol_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageObject {
    pub key: String,
    pub object_id: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub content_type: Option<String>,
    pub committed_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageObjectList {
    pub objects: Vec<StorageObject>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRepositoryStats {
    pub repository_id: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub namespace_count: u64,
    pub object_count: u64,
    pub used_bytes: u64,
    pub reserved_bytes: u64,
    pub active_upload_count: u64,
    pub pending_deletion_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn storage_request_contracts_are_closed_and_camel_case() {
        let ensure: StorageEnsureNamespaceRequest = serde_json::from_value(json!({
            "name": "recordings",
            "createdByNodeType": "camera",
            "createdByNodeId": "camera-1",
            "maxBytes": 1024,
            "overflowStrategy": "evictOldest"
        }))
        .unwrap();
        assert_eq!(
            ensure.overflow_strategy,
            StorageOverflowStrategy::EvictOldest
        );
        assert!(
            serde_json::from_value::<StorageEnsureNamespaceRequest>(json!({
                "name": "recordings",
                "createdByNodeType": "camera",
                "createdByNodeId": "camera-1",
                "maxBytes": 1024,
                "futureField": true
            }))
            .is_err()
        );

        let update: StorageNamespacePolicyUpdate = serde_json::from_value(json!({
            "namespaceId": "6f082e5e-b8bb-4d6a-bd53-8e87f58258b0",
            "expectedRevision": 2,
            "maxBytes": 2048,
            "overflowStrategy": "reject"
        }))
        .unwrap();
        assert_eq!(update.expected_revision, 2);

        let session: StorageSessionRequest = serde_json::from_value(json!({
            "namespaceId": "6f082e5e-b8bb-4d6a-bd53-8e87f58258b0",
            "permissions": ["read", "write", "delete", "list"],
            "ttlSeconds": 900
        }))
        .unwrap();
        assert_eq!(session.permissions.len(), 4);
    }

    #[test]
    fn storage_responses_are_additive_for_consumers() {
        let namespace: StorageNamespace = serde_json::from_value(json!({
            "id": "6f082e5e-b8bb-4d6a-bd53-8e87f58258b0",
            "name": "recordings",
            "createdByNodeType": "camera",
            "createdByNodeId": "camera-1",
            "maxBytes": 1024,
            "overflowStrategy": "reject",
            "usedBytes": 0,
            "reservedBytes": 0,
            "objectCount": 0,
            "evictedObjectCount": 0,
            "evictedBytes": 0,
            "rejectedWriteCount": 0,
            "revision": 1,
            "createdAtMs": 100,
            "updatedAtMs": 100,
            "futureField": true
        }))
        .unwrap();
        assert_eq!(namespace.policy().max_bytes, 1024);

        let stats: StorageRepositoryStats = serde_json::from_value(json!({
            "repositoryId": "repository-1",
            "totalBytes": 4096,
            "availableBytes": 2048,
            "namespaceCount": 1,
            "objectCount": 2,
            "usedBytes": 1024,
            "reservedBytes": 0,
            "activeUploadCount": 0,
            "pendingDeletionCount": 1,
            "futureField": true
        }))
        .unwrap();
        assert_eq!(stats.pending_deletion_count, 1);
    }

    #[test]
    fn storage_route_and_strategy_constants_are_canonical() {
        assert_eq!(STORAGE_NODE_TYPE, "storage");
        assert_eq!(STORAGE_PROTOCOL_VERSION, "storage.v1");
        assert_eq!(
            STORAGE_UPDATE_NAMESPACE_POLICY_ROUTE,
            "/storage/namespace/update-policy"
        );
        assert_eq!(StorageOverflowStrategy::Reject.as_str(), "reject");
        assert_eq!(
            StorageOverflowStrategy::parse("evictOldest").unwrap(),
            StorageOverflowStrategy::EvictOldest
        );
        assert!(StorageOverflowStrategy::parse("deleteOldest").is_err());
    }
}
