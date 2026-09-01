use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    AuthRequest, AuthenticatedSession, ServiceCoreEffect, ServiceCoreInput, ServiceCoreOutput,
};
use std::collections::HashMap;

pub const EXTERNAL_CORE_PROTOCOL_VERSION: u32 = 10;
pub const ARTIFACT_CONTENT_PATH_PREFIX: &str = "/artifact/v1/content/";

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
    PlaygroundWebhookPayload,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCallbackBaseRequest {
    pub callback_base: String,
    pub host: String,
    pub port: u16,
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
    ServiceEffects,
    HostRuntimeRequest,
    ServiceAppFacadeRequest,
    ScopeOwnedDataPurgeRequested,
    ArtifactDeliveryProjectionRequested,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalArtifactReadDescriptor {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDeliveryProjectionRequest {
    pub grant: String,
    pub expires_at_unix_ms: u64,
    pub client_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDeliveryProjectionResponse {
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
pub struct ExternalCoreServiceEffects {
    pub effects: Vec<ServiceCoreEffect>,
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
pub struct PlaygroundPayloadRequest {
    pub tenant_id: String,
    pub scope_id: String,
    pub device_id: String,
    pub capability_id: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn external_events_use_the_v10_wire_shape() {
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
        assert_eq!(EXTERNAL_CORE_PROTOCOL_VERSION, 10);
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
            grant: "payload.signature".to_string(),
            expires_at_unix_ms: 123,
            client_id: "L:client".to_string(),
        };
        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["expiresAtUnixMs"], 123);
        assert_eq!(value["clientId"], "L:client");
    }
}
