#![allow(clippy::derivable_impls, clippy::should_implement_trait)]

mod external_rpc;

pub use external_rpc::*;

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Shared MWS transport limits. Both websocket peers must apply these values so
/// an envelope accepted by one side is never rejected solely due to asymmetric
/// transport configuration.
pub const MWS_MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;
pub const MWS_MAX_FRAME_SIZE: usize = 4 * 1024 * 1024;
pub const MWS_MAX_WRITE_BUFFER_SIZE: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExceptionCode {
    Unsupported,
    NullData,
    ErrorSubscribe,
    ErrorUnsubscribe,
    ErrorProcessDataReport,
    ErrorProcessDataPoll,
    ErrorLock,
    ErrorUnlock,
    FunctionNoImpl,
    Unreachable,
    Unauthenticated,
    Unauthorized,
    PreconditionFail,
    SessionExpired,
    Internal,
    Unknown,
    NotFound,
    AlreadyExists,
    BadRequest,
    InvalidWidgetDefinition,
    DeadlineExceeded,
    TemporaryUnavailable,
    ResourceLocked,
    WsConnectionLost,
    BillAutomationExceed,
    BillTokenExceed,
    MissingContext,
}

impl ExceptionCode {
    pub fn from_str(s: &str) -> Self {
        match s {
            "UNSUPPORTED" => Self::Unsupported,
            "NULL_DATA" => Self::NullData,
            "ERROR_SUBSCRIBE" => Self::ErrorSubscribe,
            "ERROR_UNSUBSCRIBE" => Self::ErrorUnsubscribe,
            "ERROR_PROCESS_DATA_REPORT" => Self::ErrorProcessDataReport,
            "ERROR_PROCESS_DATA_POLL" => Self::ErrorProcessDataPoll,
            "ERROR_LOCK" => Self::ErrorLock,
            "ERROR_UNLOCK" => Self::ErrorUnlock,
            "FUNCTION_NO_IMPL" => Self::FunctionNoImpl,
            "UNREACHABLE" => Self::Unreachable,
            "UNAUTHENTICATED" => Self::Unauthenticated,
            "UNAUTHORIZED" => Self::Unauthorized,
            "PRECONDITION_FAIL" => Self::PreconditionFail,
            "SESSION_EXPIRED" => Self::SessionExpired,
            "INTERNAL" => Self::Internal,
            "NOT_FOUND" => Self::NotFound,
            "ALREADY_EXISTS" => Self::AlreadyExists,
            "BAD_REQUEST" => Self::BadRequest,
            "INVALID_WIDGET_DEFINITION" => Self::InvalidWidgetDefinition,
            "DEADLINE_EXCEEDED" => Self::DeadlineExceeded,
            "TEMPORARY_UNAVAILABLE" => Self::TemporaryUnavailable,
            "RESOURCE_LOCKED" => Self::ResourceLocked,
            "WS_CONNECTION_LOST" => Self::WsConnectionLost,
            "BILL_AUTOMATION_EXCEED" => Self::BillAutomationExceed,
            "BILL_TOKEN_EXCEED" => Self::BillTokenExceed,
            "MISSING_CONTEXT" => Self::MissingContext,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unsupported => "UNSUPPORTED",
            Self::NullData => "NULL_DATA",
            Self::ErrorSubscribe => "ERROR_SUBSCRIBE",
            Self::ErrorUnsubscribe => "ERROR_UNSUBSCRIBE",
            Self::ErrorProcessDataReport => "ERROR_PROCESS_DATA_REPORT",
            Self::ErrorProcessDataPoll => "ERROR_PROCESS_DATA_POLL",
            Self::ErrorLock => "ERROR_LOCK",
            Self::ErrorUnlock => "ERROR_UNLOCK",
            Self::FunctionNoImpl => "FUNCTION_NO_IMPL",
            Self::Unreachable => "UNREACHABLE",
            Self::Unauthenticated => "UNAUTHENTICATED",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::PreconditionFail => "PRECONDITION_FAIL",
            Self::SessionExpired => "SESSION_EXPIRED",
            Self::Internal => "INTERNAL",
            Self::Unknown => "UNKNOWN",
            Self::NotFound => "NOT_FOUND",
            Self::AlreadyExists => "ALREADY_EXISTS",
            Self::BadRequest => "BAD_REQUEST",
            Self::InvalidWidgetDefinition => "INVALID_WIDGET_DEFINITION",
            Self::DeadlineExceeded => "DEADLINE_EXCEEDED",
            Self::TemporaryUnavailable => "TEMPORARY_UNAVAILABLE",
            Self::ResourceLocked => "RESOURCE_LOCKED",
            Self::WsConnectionLost => "WS_CONNECTION_LOST",
            Self::BillAutomationExceed => "BILL_AUTOMATION_EXCEED",
            Self::BillTokenExceed => "BILL_TOKEN_EXCEED",
            Self::MissingContext => "MISSING_CONTEXT",
        }
    }
}

impl From<ExceptionCode> for String {
    fn from(value: ExceptionCode) -> Self {
        value.as_str().to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error_code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error_code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

pub struct ClientIds;

impl ClientIds {
    pub fn from_cloud(peer_id: &str, ws_id: &str) -> String {
        format!("C:{}:{}", peer_id, ws_id)
    }

    pub fn from_local(ws_id: &str) -> String {
        format!("L:{}", ws_id)
    }

    pub fn from_telegram(bot_id: &str, chat_id: i64) -> String {
        format!("M:telegram:{}:{}", bot_id, chat_id)
    }

    pub fn is_cloud(client_id: &str) -> bool {
        client_id.starts_with("C:")
    }

    pub fn is_local(client_id: &str) -> bool {
        client_id.starts_with("L:")
    }

    pub fn is_telegram(client_id: &str) -> bool {
        client_id.starts_with("M:telegram:")
    }

    pub fn is_messaging(client_id: &str) -> bool {
        client_id.starts_with("M:")
    }

    pub fn to_telegram_bot_id(client_id: &str) -> Option<i64> {
        let parts: Vec<&str> = client_id.splitn(6, ':').collect();
        parts.get(2)?.parse::<i64>().ok()
    }

    pub fn to_telegram_chat_id(client_id: &str) -> Option<i64> {
        let parts: Vec<&str> = client_id.splitn(6, ':').collect();
        parts.get(3)?.parse::<i64>().ok()
    }

    pub fn to_peer_id(client_id: &str) -> Option<String> {
        let parts: Vec<&str> = client_id.splitn(3, ':').collect();
        parts.get(1).map(|s| s.to_string())
    }

    pub fn to_device_id(client_id: &str) -> Option<String> {
        let parts: Vec<&str> = client_id.splitn(3, ':').collect();
        parts.get(1).map(|s| s.to_string())
    }
}

pub struct MwsMessageType;

impl MwsMessageType {
    pub const HUB_REQ: &'static str = "hrq";
    pub const HUB_RESP: &'static str = "hrp";
    pub const HUB_DATA: &'static str = "hd";
    pub const NODE_REQ: &'static str = "nrq";
    pub const NODE_RESP: &'static str = "nrp";
    pub const NODE_DATA: &'static str = "nd";
    pub const AGENT_REQ: &'static str = "grq";
    pub const AGENT_RESP: &'static str = "grp";
    pub const AGENT_DATA: &'static str = "gd";
    pub const CLOUD_REQ: &'static str = "crq";
    pub const CLOUD_RESP: &'static str = "crp";
    pub const CLOUD_DATA: &'static str = "cd";
    pub const APP_REQ: &'static str = "arq";
    pub const APP_RESP: &'static str = "arp";
    pub const SERVER_REQ: &'static str = "srq";
    pub const SERVER_RESP: &'static str = "srp";
    pub const APP_DATA: &'static str = "ad";
    pub const SERVER_DATA: &'static str = "sd";
    pub const PING: &'static str = "pi";
    pub const PONG: &'static str = "po";
}

pub struct MwsSource;

impl MwsSource {
    pub const IOS: &'static str = "ios";
    pub const ANDROID: &'static str = "android";
    pub const WINDOWS: &'static str = "windows";
    pub const MAC: &'static str = "macos";
    pub const LINUX: &'static str = "linux";
    pub const WEB: &'static str = "web";
    pub const PWA: &'static str = "pwa";
    pub const MESSAGING: &'static str = "messaging";

    pub fn is_desktop(source: &str) -> bool {
        matches!(source, Self::MAC | Self::WINDOWS | Self::LINUX)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct MwsClientInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

impl MwsClientInfo {
    pub fn new(client_id: String, user_id: String) -> Self {
        Self {
            client_id: Some(client_id),
            user_id: Some(user_id),
        }
    }

    pub fn from_ws_id(ws_id: String) -> Self {
        Self {
            client_id: Some(ClientIds::from_local(&ws_id)),
            user_id: None,
        }
    }

    pub fn from_client_id(client_id: String) -> Self {
        Self {
            client_id: Some(client_id),
            user_id: None,
        }
    }

    pub fn to_client_id(&self) -> String {
        self.client_id
            .clone()
            .unwrap_or_else(|| ClientIds::from_local("unknown"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MwsMessage {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sig: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_info: Option<MwsClientInfo>,
}

impl MwsMessage {
    pub fn create(
        scope_id: Option<String>,
        target: String,
        sig: String,
        payload: String,
        msg_type: String,
    ) -> Self {
        Self {
            scope_id,
            target: Some(target),
            sig: Some(sig),
            payload: Some(payload),
            r#type: Some(msg_type),
            from: None,
            error: None,
            client_info: None,
        }
    }

    pub fn dummy() -> Self {
        Self {
            scope_id: None,
            from: None,
            target: None,
            sig: None,
            r#type: None,
            payload: None,
            error: None,
            client_info: None,
        }
    }

    pub fn server_response(
        target: String,
        sig: String,
        payload: String,
        client_info: MwsClientInfo,
    ) -> Self {
        Self {
            scope_id: None,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::SERVER_RESP.to_string()),
            payload: Some(payload),
            error: None,
            client_info: Some(client_info),
        }
    }

    pub fn server_response_error(
        target: String,
        sig: String,
        error: ErrorResponse,
        client_info: MwsClientInfo,
    ) -> Self {
        Self {
            scope_id: None,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::SERVER_RESP.to_string()),
            payload: None,
            error: Some(error),
            client_info: Some(client_info),
        }
    }

    pub fn unscoped_server_data(
        target: String,
        payload: String,
        client_info: MwsClientInfo,
    ) -> Self {
        Self {
            scope_id: None,
            from: None,
            target: Some(target),
            sig: None,
            r#type: Some(MwsMessageType::SERVER_DATA.to_string()),
            payload: Some(payload),
            error: None,
            client_info: Some(client_info),
        }
    }

    pub fn scoped_server_data(
        scope_id: String,
        target: String,
        payload: String,
        client_info: Option<MwsClientInfo>,
    ) -> Self {
        Self {
            scope_id: Some(scope_id),
            from: None,
            target: Some(target),
            sig: None,
            r#type: Some(MwsMessageType::SERVER_DATA.to_string()),
            payload: Some(payload),
            error: None,
            client_info,
        }
    }

    pub fn hub_response(
        target: String,
        sig: String,
        payload: String,
        client_info: MwsClientInfo,
    ) -> Self {
        Self {
            scope_id: None,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::HUB_RESP.to_string()),
            payload: Some(payload),
            error: None,
            client_info: Some(client_info),
        }
    }

    pub fn hub_response_error(
        target: String,
        sig: String,
        error: ErrorResponse,
        client_info: MwsClientInfo,
    ) -> Self {
        Self {
            scope_id: None,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::HUB_RESP.to_string()),
            payload: None,
            error: Some(error),
            client_info: Some(client_info),
        }
    }

    pub fn hub_request(
        scope_id: Option<String>,
        target: String,
        sig: String,
        payload: String,
    ) -> Self {
        Self {
            scope_id,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::HUB_REQ.to_string()),
            payload: Some(payload),
            error: None,
            client_info: None,
        }
    }

    pub fn hub_data(
        scope_id: Option<String>,
        target: String,
        sig: String,
        payload: String,
    ) -> Self {
        Self {
            scope_id,
            from: None,
            target: Some(target),
            sig: Some(sig),
            r#type: Some(MwsMessageType::HUB_DATA.to_string()),
            payload: Some(payload),
            error: None,
            client_info: None,
        }
    }

    pub fn set_from(&mut self, from: String) {
        self.from = Some(from);
    }

    pub fn set_target(&mut self, target: String) {
        self.target = Some(target);
    }

    pub fn set_sig(&mut self, sig: String) {
        self.sig = Some(sig);
    }

    pub fn set_payload(&mut self, payload: String) {
        self.payload = Some(payload);
    }

    pub fn set_type(&mut self, msg_type: String) {
        self.r#type = Some(msg_type);
    }

    pub fn set_client_info(&mut self, client_info: MwsClientInfo) {
        self.client_info = Some(client_info);
    }
}

impl Default for MwsMessage {
    fn default() -> Self {
        Self::dummy()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAuthRequest {
    pub hub_id: String,
    pub token: String,
    pub node_type: String,
    #[serde(default)]
    pub node_id: String,
    #[serde(default)]
    pub instance_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeAuthResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenIssueRequest {
    pub node_type: String,
    pub candidate_host_id: String,
    pub candidate_fingerprint: String,
    pub challenge: NodeChallengeEvidence,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NodeInstancePolicy {
    Multiple,
    Singleton,
    Shared,
}

impl NodeInstancePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Multiple => "multiple",
            Self::Singleton => "singleton",
            Self::Shared => "shared",
        }
    }
}

impl Default for NodeInstancePolicy {
    fn default() -> Self {
        Self::Multiple
    }
}

impl std::fmt::Display for NodeInstancePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NodeInstancePolicy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "multiple" | "Multiple" => Ok(Self::Multiple),
            "singleton" | "Singleton" => Ok(Self::Singleton),
            "shared" | "Shared" => Ok(Self::Shared),
            other => Err(format!("unsupported node instance policy: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NodeBindingStatus {
    Pending,
    Active,
    Revoked,
    Expired,
    Failed,
}

impl NodeBindingStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Revoked => "revoked",
            Self::Expired => "expired",
            Self::Failed => "failed",
        }
    }
}

impl Default for NodeBindingStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl std::fmt::Display for NodeBindingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for NodeBindingStatus {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "pending" | "Pending" => Ok(Self::Pending),
            "active" | "Active" => Ok(Self::Active),
            "revoked" | "Revoked" => Ok(Self::Revoked),
            "expired" | "Expired" => Ok(Self::Expired),
            "failed" | "Failed" => Ok(Self::Failed),
            other => Err(format!("unsupported node binding status: {other}")),
        }
    }
}

pub const NODE_ONBOARDING_CHALLENGE_PROTOCOL: &str = "meow.node.onboarding.challenge";
pub const NODE_ONBOARDING_CHALLENGE_AUDIENCE: &str = "meow-core:node-onboarding";
pub const NODE_ONBOARDING_CHALLENGE_ALGORITHM: &str = "Ed25519";
pub const NODE_ONBOARDING_TOKEN_ISSUER_PREFIX: &str = "meow-core:hub:";
pub const NODE_ONBOARDING_TOKEN_AUDIENCE: &str = "meow-node:onboarding";
pub const NODE_ONBOARDING_ES256_ALGORITHM: &str = "ES256";
pub const NODE_ONBOARDING_TRANSACTION_TTL_SECONDS: u64 = 5 * 60;
pub const HUB_CONNECTION_PROOF_TARGET: &str = "/identity/hub/prove";
pub const HUB_CONNECTION_PROOF_PROTOCOL: &str = "meow.hub.connection.proof.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubConnectionProofRequest {
    pub protocol: String,
    pub hub_id: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HubConnectionProofResponse {
    pub protocol: String,
    pub hub_id: String,
    pub tenant_id: String,
    pub scope_id: String,
    pub nonce: String,
    pub key_id: String,
    pub signature: String,
}

/// Canonical, domain-separated bytes signed by the Hub for one physical WS connection.
/// Length prefixes keep the encoding unambiguous without relying on JSON object ordering.
pub fn hub_connection_proof_signing_payload(request: &HubConnectionProofRequest) -> Vec<u8> {
    let fields = [
        request.protocol.as_str(),
        request.hub_id.as_str(),
        request.tenant_id.as_str(),
        request.scope_id.as_str(),
        request.nonce.as_str(),
    ];
    let mut payload = Vec::new();
    for field in fields {
        payload.extend_from_slice(&(field.len() as u64).to_be_bytes());
        payload.extend_from_slice(field.as_bytes());
    }
    payload
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeChallengeEvidence {
    pub protocol: String,
    pub algorithm: String,
    pub payload: String,
    pub signature: String,
    pub public_key: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeChallengePayload {
    pub protocol: String,
    pub aud: String,
    pub nonce: String,
    pub scope_id: String,
    pub node_type: String,
    pub host_id: String,
    pub fingerprint: String,
    pub service_instance_id: String,
    pub instance_policy: NodeInstancePolicy,
    pub instance_slot: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenIssueResponse {
    pub token: String,
    pub node_id: String,
    pub expires_in: i64,
    pub hub_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenRevokeRequest {
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenRevokeResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenListItem {
    pub node_id: String,
    pub node_type: String,
    pub hub_id: String,
    pub host_id: String,
    pub fingerprint: String,
    pub service_instance_id: String,
    pub instance_policy: NodeInstancePolicy,
    pub instance_slot: String,
    pub status: NodeBindingStatus,
    pub issued_at: i64,
    pub expires_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeTokenListResponse {
    #[serde(default)]
    pub nodes: Vec<NodeTokenListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInstanceListItem {
    pub node_id: String,
    pub node_type: String,
    pub hub_id: String,
    pub host_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_name: Option<String>,
    pub fingerprint: String,
    pub service_instance_id: String,
    pub instance_policy: NodeInstancePolicy,
    pub instance_slot: String,
    pub binding_status: NodeBindingStatus,
    pub issued_at: i64,
    pub expires_at: i64,
    pub connected: bool,
    pub runtime_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeInstanceListResponse {
    #[serde(default)]
    pub instances: Vec<NodeInstanceListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticatedSession {
    pub tenant_id: String,
    pub scope_id: String,
    pub user_id: String,
    pub is_test: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base64: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

impl Image {
    pub fn from_base64(base64: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self {
            base64: Some(base64.into()),
            mime_type: Some(mime_type.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolCallFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChatToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: ChatToolCallFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<Image>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ChatToolCall>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmGenerationOptions {
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmCompleteRequest {
    #[serde(default)]
    pub use_case: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    pub messages: Vec<ChatMessage>,
    #[serde(default)]
    pub tools: Option<serde_json::Value>,
    #[serde(default)]
    pub provider: Option<serde_json::Value>,
    #[serde(default)]
    pub response_format: Option<serde_json::Value>,
    #[serde(default)]
    pub options: LlmGenerationOptions,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmRouteInfo {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LlmCompleteResponse {
    pub content: String,
    #[serde(default)]
    pub tool_calls: Vec<ProviderToolCall>,
    #[serde(default)]
    pub route: Option<LlmRouteInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceCoreInput {
    ClientAuth {
        message: MwsMessage,
        ws_id: String,
        scope_id: String,
    },
    ClientRequest {
        target: String,
        payload: String,
        ws_id: String,
        tenant_id: String,
        scope_id: String,
        user_id: String,
        is_test: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        surface_id: Option<String>,
    },
    /// Invokes the canonical Conversation application boundary without a
    /// websocket/client transport identity.
    ConversationRequest {
        target: String,
        payload: String,
        tenant_id: String,
        scope_id: String,
        user_id: String,
        surface_id: String,
        is_test: bool,
    },
    NodeAuth {
        request: NodeAuthRequest,
    },
    HubConnectionProof {
        request: HubConnectionProofRequest,
    },
    NodeRequest {
        target: String,
        payload: String,
        tenant_id: String,
        scope_id: String,
        node_type: String,
        instance_id: String,
    },
    BackendStatusChanged {
        tenant_id: String,
        scope_id: String,
        target: String,
        payload: String,
    },
    ClientResponse {
        message: MwsMessage,
    },
    NodeResponse {
        message: MwsMessage,
    },
    IssueLocalSessionToken {
        tenant_id: String,
        scope_id: String,
        user_id: String,
        app_client_id: String,
    },
    SetLocalAppClientFocus {
        ws_id: String,
        focused: bool,
    },
    RefreshLocalAppClient {
        ws_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceCoreResponse {
    ClientAuth {
        session: Option<AuthenticatedSession>,
        response: MwsMessage,
    },
    ClientRequest {
        response_payload: Option<String>,
        client_info: MwsClientInfo,
    },
    ConversationRequest {
        response_payload: Option<String>,
    },
    NodeAuth(NodeAuthResponse),
    HubConnectionProof(HubConnectionProofResponse),
    NodeRequest {
        response_payload: Option<String>,
    },
    LocalSessionToken(String),
    Ack {
        handled: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceCoreEffect {
    Local {
        client_id: String,
        message: MwsMessage,
    },
    Bridge {
        tenant_id: String,
        scope_id: String,
        message: MwsMessage,
    },
    Node {
        connection_key: String,
        message: MwsMessage,
    },
    NodeDisconnect {
        connection_key: String,
    },
    Surface {
        tenant_id: String,
        scope_id: String,
        user_id: String,
        surface_id: String,
        target: String,
        payload: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceCoreOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<ServiceCoreResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<ServiceCoreEffect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSetting {
    #[serde(default)]
    pub script_mode: bool,
    #[serde(default)]
    pub debug_mode: bool,
    #[serde(default)]
    pub eng_account: bool,
}

impl UserSetting {
    pub fn new() -> Self {
        Self {
            script_mode: false,
            debug_mode: false,
            eng_account: false,
        }
    }
}

impl Default for UserSetting {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setting: Option<UserSetting>,
    #[serde(default)]
    pub eng: bool,
}

impl UserInfo {
    pub fn new(id: String) -> Self {
        Self {
            id: Some(id),
            name: None,
            email: None,
            setting: None,
            eng: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeMember {
    pub id: Option<String>,
    pub email: Option<String>,
    pub name: Option<String>,
    pub pending: bool,
    pub role: Option<String>,
}

impl Default for ScopeMember {
    fn default() -> Self {
        Self {
            id: None,
            email: None,
            name: None,
            pending: false,
            role: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeInfo {
    pub name: Option<String>,
    pub id: Option<String>,
    pub pending: bool,
    pub members: Option<Vec<ScopeMember>>,
    pub execution_env: Option<String>,
    pub mode: Option<String>,
    pub connection_mode: Option<String>,
    pub agent_mode: Option<String>,
    pub default_active: bool,
    #[serde(default)]
    pub is_test: bool,
}

impl Default for ScopeInfo {
    fn default() -> Self {
        Self {
            name: None,
            id: None,
            pending: false,
            members: None,
            execution_env: None,
            mode: None,
            connection_mode: None,
            agent_mode: None,
            default_active: false,
            is_test: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    pub tenant_id: String,
    pub heartbeat_interval: i32,
    pub command_timeout: i32,
    pub user_info: UserInfo,
    pub scope: ScopeInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_token: Option<String>,
}

impl AuthConfig {
    pub fn new(
        tenant_id: String,
        heartbeat_interval: i32,
        command_timeout: i32,
        user_info: UserInfo,
        scope: ScopeInfo,
    ) -> Self {
        Self {
            tenant_id,
            heartbeat_interval,
            command_timeout,
            user_info,
            scope,
            hub_id: None,
            jwt_token: None,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            tenant_id: String::new(),
            heartbeat_interval: 240,
            command_timeout: 10,
            user_info: UserInfo::new(String::new()),
            scope: ScopeInfo::default(),
            hub_id: None,
            jwt_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequest {
    pub token: String,
    pub source: String,
    pub scope_id: String,
    pub device_id: String,
    pub client_source: String,
    #[serde(default)]
    pub tenant_id: Option<String>,
    #[serde(default)]
    pub hub_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubMdnsInstanceRecord {
    pub tenant_id: String,
    pub scope_id: String,
    pub hub_id: String,
    pub scope_name: String,
}

#[cfg(test)]
mod tests {
    use super::{
        hub_connection_proof_signing_payload, HubConnectionProofRequest, MwsMessage,
        MwsMessageType, HUB_CONNECTION_PROOF_PROTOCOL,
    };

    #[test]
    fn hub_connection_proof_payload_is_unambiguous_and_nonce_bound() {
        let request = HubConnectionProofRequest {
            protocol: HUB_CONNECTION_PROOF_PROTOCOL.to_string(),
            hub_id: "hub-1".to_string(),
            tenant_id: "tenant-1".to_string(),
            scope_id: "scope-1".to_string(),
            nonce: "nonce-1".to_string(),
        };
        let payload = hub_connection_proof_signing_payload(&request);
        let mut changed = request.clone();
        changed.nonce = "nonce-2".to_string();

        assert_ne!(payload, hub_connection_proof_signing_payload(&changed));
        assert!(payload.starts_with(&(HUB_CONNECTION_PROOF_PROTOCOL.len() as u64).to_be_bytes()));
    }

    #[test]
    fn scoped_server_data_builds_scope_envelope() {
        let message = MwsMessage::scoped_server_data(
            "scope-1".to_string(),
            "/dialog".to_string(),
            "{}".to_string(),
            None,
        );

        assert_eq!(message.scope_id.as_deref(), Some("scope-1"));
        assert_eq!(message.target.as_deref(), Some("/dialog"));
        assert_eq!(message.r#type.as_deref(), Some(MwsMessageType::SERVER_DATA));
        assert_eq!(message.payload.as_deref(), Some("{}"));
        assert!(message.client_info.is_none());
    }
}
