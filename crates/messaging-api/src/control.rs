use crate::ConversationAudience;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Placement {
    Local,
    Cloud,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrontendProvider {
    pub provider: String,
    pub deprecated: bool,
    pub supported_placements: Vec<Placement>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderListRequest {
    pub placement: Placement,
    pub frontend_providers: Vec<FrontendProvider>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderMetadata {
    pub provider: String,
    pub display_name: String,
    pub placement: Placement,
    pub deprecated: bool,
    pub available: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled_reason: Option<String>,
    pub setup_flow: String,
    pub audiences: Vec<ConversationAudience>,
    pub inbound_capabilities: Vec<ContentCapability>,
    pub outbound_capabilities: Vec<ContentCapability>,
    pub delivery_mode: DeliveryMode,
    pub management_operations: Vec<ManagementOperation>,
    pub provider_data: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentCapability {
    Text,
    Markdown,
    Image,
    Audio,
    Action,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    Unrestricted,
    SessionWindow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManagementOperation {
    #[serde(rename = "connection.list")]
    ConnectionList,
    #[serde(rename = "connection.update")]
    ConnectionUpdate,
    #[serde(rename = "connection.delete")]
    ConnectionDelete,
    #[serde(rename = "connection.test")]
    ConnectionTest,
    #[serde(rename = "connection.status")]
    ConnectionStatus,
    #[serde(rename = "account.bind")]
    AccountBind,
    #[serde(rename = "account.unbind")]
    AccountUnbind,
    #[serde(rename = "account.binding.list")]
    AccountBindingList,
    #[serde(rename = "setup.options")]
    SetupOptions,
    #[serde(rename = "setup.start")]
    SetupStart,
    #[serde(rename = "setup.status")]
    SetupStatus,
    #[serde(rename = "surface.list")]
    SurfaceList,
    #[serde(rename = "surface.dismiss")]
    SurfaceDismiss,
    #[serde(rename = "surface.bind_code.create")]
    SurfaceBindCodeCreate,
    #[serde(rename = "actor.link_code.create")]
    ActorLinkCodeCreate,
    #[serde(rename = "actor.link.list")]
    ActorLinkList,
    #[serde(rename = "actor.link.delete")]
    ActorLinkDelete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderListResponse {
    pub providers: Vec<ProviderMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderPlacementRequest {
    pub provider: String,
    pub placement: Placement,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionUpdateRequest {
    pub provider: String,
    pub placement: Placement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionRequest {
    pub provider: String,
    pub placement: Placement,
    pub connection_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionTestRequest {
    pub provider: String,
    pub placement: Placement,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingConnection {
    pub provider: String,
    pub placement: Placement,
    pub connection_id: String,
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub enabled: bool,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
    pub bound_to_current_scope: bool,
    pub provider_data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionListResponse {
    pub connections: Vec<MessagingConnection>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionUpdateResponse {
    pub connection: MessagingConnection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionTestResponse {
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderStatus {
    pub provider: String,
    pub placement: Placement,
    pub enabled: bool,
    pub provider_data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConnectionStatusResponse {
    pub status: ProviderStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MutationResponse {
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountBindingRequest {
    pub provider: String,
    pub placement: Placement,
    pub account_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountBinding {
    pub tenant_id: String,
    pub scope_id: String,
    pub user_id: String,
    pub default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AccountBindingListResponse {
    pub bindings: Vec<AccountBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupStartRequest {
    pub provider: String,
    pub placement: Placement,
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupOptionsResponse {
    pub options: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupStatusRequest {
    pub provider: String,
    pub placement: Placement,
    pub setup_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupAction {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub setup_id: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_ms: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<SetupAction>,
    pub provider_data: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupResponse {
    pub setup: SetupState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SurfaceState {
    Pending,
    Bound,
    Dismissed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceListRequest {
    pub provider: String,
    pub placement: Placement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<SurfaceState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MessagingSurface {
    pub provider: String,
    pub placement: Placement,
    pub account_id: String,
    pub surface_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub state: SurfaceState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    pub first_seen_at_ms: i64,
    pub last_seen_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceListResponse {
    pub surfaces: Vec<MessagingSurface>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceRequest {
    pub provider: String,
    pub placement: Placement,
    pub surface_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SurfaceBindCodeCreateRequest {
    pub provider: String,
    pub placement: Placement,
    pub surface_id: String,
    pub scope_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLinkCodeCreateRequest {
    pub provider: String,
    pub placement: Placement,
    pub surface_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChallengeCode {
    pub code: String,
    pub command: String,
    pub expires_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChallengeCodeResponse {
    pub challenge: ChallengeCode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLinkListRequest {
    pub provider: String,
    pub placement: Placement,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLinkView {
    pub link_id: String,
    pub provider: String,
    pub placement: Placement,
    pub account_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLinkListResponse {
    pub links: Vec<ActorLinkView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ActorLinkDeleteRequest {
    pub provider: String,
    pub placement: Placement,
    pub link_id: String,
}
