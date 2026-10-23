use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{AuthRequest, AuthenticatedSession, ServiceCoreInput, ServiceCoreOutput};
use std::collections::HashMap;

pub const EXTERNAL_CORE_PROTOCOL_VERSION: u32 = 22;
/// Maximum single native artifact body, separate from the JSON control frame limit.
pub const MAX_NATIVE_ARTIFACT_BYTES: usize = 50 * 1024 * 1024;
pub const MAX_NATIVE_ARTIFACT_BASE64_BYTES: usize = MAX_NATIVE_ARTIFACT_BYTES.div_ceil(3) * 4;
/// Protocol 18 sends large dataBase64 strings after the JSON frame. The JSON field
/// is an empty string marker, followed by a u32 byte length and that many ASCII
/// bytes on the same connection. Only artifact commit requests and open/read
/// responses support this attachment. Control frames keep their 16 MiB limit.
pub const ARTIFACT_ATTACHMENT_CHUNK_BYTES: usize = 1024 * 1024;
pub const ARTIFACT_CONTENT_PATH_PREFIX: &str = "/artifact/v1/content/";
pub const ARTIFACT_UPLOAD_PATH_PREFIX: &str = "/artifact/v1/upload/";

/// Derive a bounded, workdir-specific Windows IPC name from the metadata socket path.
/// The metadata itself stays in the ordinary runtime directory on every platform.
pub fn external_core_pipe_name(socket_path: &std::path::Path) -> String {
    use sha2::{Digest, Sha256};
    let normalized = socket_path
        .to_string_lossy()
        .replace('/', "\\")
        .to_lowercase();
    format!(
        r"\\.\pipe\meowcore-{:x}",
        Sha256::digest(normalized.as_bytes())
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCoreRequest {
    pub id: String,
    pub method: ExternalCoreMethod,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCoreResponse {
    pub id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ExternalCoreError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalCoreError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl From<crate::ErrorResponse> for ExternalCoreError {
    fn from(error: crate::ErrorResponse) -> Self {
        Self {
            code: error.error,
            message: error.message,
            details: error.details,
        }
    }
}

impl From<ExternalCoreError> for crate::ErrorResponse {
    fn from(error: ExternalCoreError) -> Self {
        Self {
            error: error.code,
            message: error.message,
            details: error.details,
        }
    }
}

impl std::fmt::Display for ExternalCoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

impl std::error::Error for ExternalCoreError {}

/// A completed JSON HTTP handler response, including non-2xx responses.
/// This is an RPC result, not an RPC error. Hosts must preserve both fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpPayloadResponse {
    pub status_code: u16,
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExternalCoreMethod {
    HandleCoreInput,
    RegisterLocalAppClient,
    RegisterNodeConnection,
    CleanupWsState,
    UpdateCallbackBase,
    PutArtifact,
    OpenArtifact,
    AuthorizeArtifactRead,
    AuthorizeArtifactUpload,
    CommitArtifactUpload,
    StreamArtifactUpload,
    DeviceIdentity,
    CommissionFingerprint,
    CommissionPublicKeyBase64,
    ListMdnsRecords,
    PollEvents,
    CompleteEvents,
    CommissionChallengePayload,
    PairingStartPayload,
    SetupStartPayload,
    GeneralWebhookPayload,
    InvokeWasmPayload,
    Health,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCoreInputRequest {
    pub input: ServiceCoreInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandleCoreInputResponse {
    pub output: ServiceCoreOutput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterLocalAppClientRequest {
    pub ws_id: String,
    pub auth_request: AuthRequest,
    pub session: AuthenticatedSession,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterNodeConnectionRequest {
    pub binding_metadata: crate::NodeBindingMetadata,
    pub ws_id: String,
    pub session_id: String,
    pub node_type: String,
    pub node_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    pub tenant_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupWsStateRequest {
    pub ws_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateCallbackBaseRequest {
    /// Host observation, never an independently discovered or fallback address.
    pub network: crate::host::network::HostNetworkSnapshot,
    /// Actual bound native HTTPS listener port. Zero means it is not ready.
    pub port: u16,
    /// Restricted third-party HTTP listener. Zero disables compatibility URLs.
    #[serde(default)]
    pub third_party_port: u16,
}

impl UpdateCallbackBaseRequest {
    pub fn callback_base(&self, now_ms: i64) -> Option<String> {
        if self.third_party_port == 0 {
            return None;
        }
        self.network
            .available_ipv4(now_ms)
            .map(|ip| format!("http://{ip}:{}", self.third_party_port))
    }
    pub fn native_base(&self, now_ms: i64) -> Option<String> {
        if self.port == 0 {
            return None;
        }
        self.network
            .available_ipv4(now_ms)
            .map(|ip| format!("https://{ip}:{}", self.port))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreDeviceIdentity {
    pub device_id: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListMdnsRecordsRequest {
    pub port: u16,
    pub addresses: Vec<String>,
    pub host_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreMdnsRecord {
    pub service_type: String,
    pub instance_name: String,
    pub port: u16,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollExternalCoreEventsRequest {
    pub consumer_id: String,
    pub max_events: u16,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollExternalCoreEventsResponse {
    pub events: Vec<ExternalCoreEvent>,
    pub timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteExternalCoreEventsRequest {
    pub consumer_id: String,
    pub completions: Vec<ExternalCoreEventCompletion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteExternalCoreEventsResponse {
    pub completed_event_ids: Vec<String>,
    pub missing_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreEvent {
    pub event_id: String,
    pub kind: ExternalCoreEventKind,
    pub payload: Value,
    pub expects_response: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExternalCoreEventKind {
    MdnsRecordsChanged,
    DeliveryRequested,
    ConnectionControlRequested,
    HostRuntimeRequest,
    ServiceAppFacadeRequest,
    ScopeOwnedDataPurgeRequested,
    MessagingRelationshipsRequested,
    ArtifactDeliveryProjectionRequested,
    ArtifactUploadProjectionRequested,
}

/// Messaging-only Core -> Host request. A successful response means delivery was accepted by
/// the provider implementation, not merely queued in the Core event pump or read by a user.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingDeliveryRequest {
    pub tenant_id: String,
    pub scope_id: String,
    pub surface_id: String,
    pub target: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingDeliveryResponse {
    pub outcome: crate::DeliveryOutcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalArtifactRequestContext {
    pub tenant_id: String,
    pub scope_id: String,
    pub actor_user_id: String,
    pub client_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalPutArtifactRequest {
    pub context: ExternalArtifactRequestContext,
    pub artifact: artifact_api::PutArtifactRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalOpenArtifactRequest {
    pub context: ExternalArtifactRequestContext,
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalAuthorizeArtifactReadRequest {
    pub grant: String,
    /// Inspect existence and metadata without reading content or renewing idle expiry.
    /// The response has neither a file path nor inline content in this mode.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub metadata_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalAuthorizeArtifactUploadRequest {
    pub grant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalCommitArtifactUploadRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    pub grant: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalArtifactReadDescriptor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_base64: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub temporary: bool,
    pub path: String,
    pub kind: artifact_api::ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalArtifactUploadDescriptor {
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub temporary: bool,
    pub upload_id: String,
    pub path: String,
    pub kind: artifact_api::ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDeliveryProjectionRequest {
    /// Explicit compatibility request; native traffic must remain HTTPS.
    #[serde(default)]
    pub third_party: bool,
    pub grant: String,
    pub expires_at_unix_ms: u64,
    pub client_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDeliveryProjectionResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_identity: Option<artifact_api::HostTlsIdentity>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactUploadProjectionRequest {
    pub grant: String,
    pub expires_at_unix_ms: u64,
    pub client_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactUploadProjectionResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_identity: Option<artifact_api::HostTlsIdentity>,
    pub url: String,
}

/// An App Facade request delegated by an externally hosted Core to the
/// application service that owns the target's business logic.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalServiceAppFacadeRequest {
    pub target: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub user_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    pub payload: Value,
}

/// A platform capability request emitted by an externally hosted Core.
///
/// Core owns the onboarding workflow. The native host owns LAN discovery and
/// transport to a discovered candidate, so Android can satisfy this contract
/// without moving application logic into the mobile shell.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalHostRuntimeRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_public_key: Option<crate::host::auth::PublicKey>,
    pub method: ExternalHostRuntimeMethod,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExternalHostRuntimeMethod {
    Discovery,
    CandidateRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScopeOwnedDataPurgeRequest {
    pub tenant_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreEventCompletion {
    pub event_id: String,
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ExternalCoreError>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpPayloadRequest {
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointPayloadRequest {
    pub endpoint_id: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreRuntimeMetadata {
    pub backend: String,
    pub instance_id: String,
    pub pid: u32,
    pub state: String,
    pub started_at: String,
    pub protocol_version: u32,
    pub binary_version: String,
    pub socket_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalCoreHealth {
    pub runtime_space_policy: Option<crate::runtime_space::RuntimeSpacePolicy>,
    pub ok: bool,
    pub ready: bool,
    pub backend: String,
    pub binary_version: String,
    pub agent_version: String,
    pub protocol_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default)]
    pub capabilities: Vec<String>,
}

impl ExternalArtifactReadDescriptor {
    /// Decode the bounded inline variant. File descriptors return None.
    pub fn inline_content(&self) -> Result<Option<Vec<u8>>, String> {
        use base64::Engine as _;
        use sha2::{Digest, Sha256};
        let Some(encoded) = &self.data_base64 else {
            return if self.path.is_empty() {
                Err("artifact descriptor has no content".into())
            } else {
                Ok(None)
            };
        };
        if !self.path.is_empty()
            || self.size_bytes > MAX_NATIVE_ARTIFACT_BYTES as u64
            || encoded.len() > MAX_NATIVE_ARTIFACT_BASE64_BYTES
        {
            return Err("invalid inline artifact descriptor".into());
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| "invalid inline artifact base64")?;
        if bytes.len() as u64 != self.size_bytes
            || format!("{:x}", Sha256::digest(&bytes)) != self.sha256
        {
            return Err("inline artifact failed its integrity check".into());
        }
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_address_requires_a_fresh_host_observation_and_bound_port() {
        let mut request = UpdateCallbackBaseRequest {
            third_party_port: 0,
            network: crate::host::network::HostNetworkSnapshot {
                lan_ipv4: Some("192.168.1.5".parse().unwrap()),
                observed_at_ms: 100_000,
            },
            port: 3210,
        };
        assert_eq!(
            request.native_base(100_000).as_deref(),
            Some("https://192.168.1.5:3210")
        );
        let roundtrip: UpdateCallbackBaseRequest =
            serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert_eq!(request, roundtrip);
        assert_eq!(request.callback_base(100_000), None);
        request.third_party_port = 3211;
        assert_eq!(
            request.callback_base(100_000).as_deref(),
            Some("http://192.168.1.5:3211")
        );
        assert_eq!(request.callback_base(145_000), None);
        assert_eq!(request.native_base(145_000), None);
        request.port = 0;
        assert_eq!(request.native_base(100_000), None);
        assert_eq!(
            UpdateCallbackBaseRequest::default().native_base(100_000),
            None
        );
    }

    #[test]
    fn messaging_delivery_is_a_closed_request_response_contract() {
        let request = serde_json::json!({"tenantId":"t", "scopeId":"s", "surfaceId":"m",
            "target":"/chat/event", "payload":"{}"});
        let decoded: MessagingDeliveryRequest = serde_json::from_value(request.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), request);
        let mut invalid = request;
        invalid["unknown"] = serde_json::json!(true);
        assert!(serde_json::from_value::<MessagingDeliveryRequest>(invalid).is_err());
        assert!(
            serde_json::from_value::<MessagingDeliveryResponse>(serde_json::json!({})).is_err()
        );
        assert_eq!(
            serde_json::to_value(ExternalCoreEventKind::DeliveryRequested).unwrap(),
            "deliveryRequested"
        );
    }

    #[test]
    fn artifact_read_metadata_mode_is_explicit_and_defaults_to_content() {
        let read: ExternalAuthorizeArtifactReadRequest =
            serde_json::from_value(serde_json::json!({"grant":"signed"})).unwrap();
        assert!(!read.metadata_only);
        assert_eq!(
            serde_json::to_value(read).unwrap(),
            serde_json::json!({"grant":"signed"})
        );
        let head: ExternalAuthorizeArtifactReadRequest =
            serde_json::from_value(serde_json::json!({"grant":"signed", "metadataOnly":true}))
                .unwrap();
        assert!(head.metadata_only);
        assert!(
            serde_json::from_value::<ExternalAuthorizeArtifactReadRequest>(
                serde_json::json!({"grant":"signed", "metadataOnly":"true"})
            )
            .is_err()
        );
    }

    #[test]
    fn external_events_use_the_current_wire_shape() {
        let event = ExternalCoreEvent {
            event_id: "event-1".to_string(),
            kind: ExternalCoreEventKind::ScopeOwnedDataPurgeRequested,
            payload: serde_json::to_value(ScopeOwnedDataPurgeRequest {
                tenant_id: "tenant-1".to_string(),
                scope_id: "scope-1".to_string(),
            })
            .unwrap(),
            expects_response: true,
        };

        let value = serde_json::to_value(event).unwrap();
        assert_eq!(EXTERNAL_CORE_PROTOCOL_VERSION, 22);
        assert_eq!(value["kind"], "scopeOwnedDataPurgeRequested");
        assert_eq!(value["payload"]["tenantId"], "tenant-1");
        assert_eq!(value["payload"]["scopeId"], "scope-1");
        assert_eq!(value["expectsResponse"], true);

        let facade = ExternalCoreEvent {
            event_id: "event-2".to_string(),
            kind: ExternalCoreEventKind::ServiceAppFacadeRequest,
            payload: serde_json::to_value(ExternalServiceAppFacadeRequest {
                target: "/app/messaging/provider/list".to_string(),
                tenant_id: "tenant-1".to_string(),
                scope_id: "scope-1".to_string(),
                user_id: "user-1".to_string(),
                client_id: None,
                payload: serde_json::json!({"placement": "local"}),
            })
            .unwrap(),
            expects_response: true,
        };
        let value = serde_json::to_value(facade).unwrap();
        assert_eq!(value["kind"], "serviceAppFacadeRequest");
        assert_eq!(value["payload"]["target"], "/app/messaging/provider/list");
        assert_eq!(value["payload"]["userId"], "user-1");
        assert!(value["payload"].get("clientId").is_none());
    }

    #[test]
    fn artifact_host_contract_uses_canonical_wire_types() {
        let descriptor = ExternalArtifactReadDescriptor {
            temporary: false,
            data_base64: None,
            path: "/runtime/artifacts/blob".to_string(),
            kind: artifact_api::ArtifactKind::Audio,
            mime_type: "audio/ogg".to_string(),
            size_bytes: 42,
            width: None,
            height: None,
            duration_millis: Some(1_500),
            sha256: "a".repeat(64),
        };
        let value = serde_json::to_value(descriptor).unwrap();
        assert_eq!(value["kind"], "AUDIO");
        assert_eq!(value["mimeType"], "audio/ogg");
        assert_eq!(value["durationMillis"], 1_500);
        assert!(value.get("width").is_none());

        let request = ArtifactDeliveryProjectionRequest {
            third_party: false,
            grant: "payload.signature".to_string(),
            expires_at_unix_ms: 123,
            client_id: "L:client".to_string(),
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["expiresAtUnixMs"], 123);
        assert_eq!(value["clientId"], "L:client");
    }

    #[test]
    fn playground_uses_only_the_general_webhook_contract() {
        let general = serde_json::to_value(ExternalCoreMethod::GeneralWebhookPayload).unwrap();
        assert_eq!(general, serde_json::json!("GeneralWebhookPayload"));
        assert!(
            serde_json::from_value::<ExternalCoreMethod>(serde_json::json!(
                "PlaygroundWebhookPayload"
            ))
            .is_err()
        );
    }
}

/// Host-to-Core bounded transfer commands. Public clients send a binary HTTP body.
/// Only the owner keeps upload state, digest and staging content.
pub const ARTIFACT_STREAM_CHUNK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExternalArtifactStreamRequest {
    Begin { grant: String, mime_type: String },
    Append { grant: String, data_base64: String },
    Finish { grant: String },
    Abort { grant: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum ExternalArtifactStreamResponse {
    Receiving {
        max_size_bytes: u64,
    },
    Accepted {},
    Staged {
        receipt: artifact_api::ArtifactUploadReceipt,
    },
}
