//! SDK-owned object transfer control. Folder authority and publication remain in Core.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
pub enum StorageObjectControl {
    Prepare {
        #[serde(rename = "sizeBytes")]
        size_bytes: u64,
        sha256: String,
        #[serde(rename = "contentType")]
        content_type: Option<String>,
        #[serde(rename = "localImport", default)]
        local_import: bool,
    },
    Complete {
        #[serde(rename = "uploadId")]
        upload_id: String,
    },
    Status {
        #[serde(rename = "uploadId")]
        upload_id: String,
    },
    Abort {
        #[serde(rename = "uploadId")]
        upload_id: String,
    },
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageLocalEndpoint {
    pub socket: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageTransfer {
    pub url: String,
    pub token: String,
    pub host_identity: Option<artifact_api::HostTlsIdentity>,
    pub expires_at_ms: u64,
    pub local: Option<StorageLocalEndpoint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageUploadState {
    Prepared,
    Published,
    Aborted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageUpload {
    pub upload_id: String,
    pub state: StorageUploadState,
    pub object: Option<storage_api::StorageObject>,
    pub transfer: Option<StorageTransfer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageReadTransfer {
    pub object: storage_api::StorageObject,
    pub transfer: StorageTransfer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum StorageLocalOperation {
    Write,
    Import,
    Read,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StorageLocalRequest {
    pub token: String,
    pub operation: StorageLocalOperation,
    pub range: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "camelCase")]
pub enum StorageLocalReply {
    Ready {
        blob: storage_api::backend::Blob,
        range: Option<(u64, u64)>,
    },
    Done,
    Error {
        error: storage_api::StorageError,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn transfer_control_does_not_accept_a_caller_selected_repository_or_host() {
        let input = json!({"action":"prepare","sizeBytes":5,"sha256":"a".repeat(64),"contentType":"video/mp4","localImport":true});
        let request: StorageObjectControl = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), input);
        for field in ["repositoryId", "hostId", "nodeId", "url"] {
            let mut forged = input.clone();
            forged[field] = json!("caller-selected");
            assert!(serde_json::from_value::<StorageObjectControl>(forged).is_err());
        }
        for action in ["complete", "status", "abort"] {
            assert!(
                serde_json::from_value::<StorageObjectControl>(json!({"action":action})).is_err()
            );
            let input = json!({"action":action,"uploadId":"operation"});
            assert_eq!(
                serde_json::to_value(
                    serde_json::from_value::<StorageObjectControl>(input.clone()).unwrap()
                )
                .unwrap(),
                input
            );
        }
    }
}
