//! Public LLM settings. Catalog facts, saved configuration and active mode are distinct.
//! Credentials and runtime transport configuration never belong to these DTOs.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmMode {
    Cloud,
    Local,
}
impl LlmMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cloud => "cloud",
            Self::Local => "local",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListProvidersInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_mode: Option<LlmMode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GetProviderInput {
    pub provider: String,
    pub llm_mode: LlmMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub provider: String,
    pub display_name: String,
    pub runtime: String,
    pub model_source: String,
    pub auth_kind: String,
    pub use_cases: Vec<String>,
    pub allowed_agent_modes: HashMap<String, Vec<String>>,
    pub provider_metadata: Option<Value>,
    /// Space-owned local configuration, absent for platform-managed cloud providers.
    pub configuration: Option<ProviderConfiguration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderConfiguration {
    pub requires_credential: bool,
    /// Stored credentials exist; this is not a successful login/network check.
    pub has_credential: bool,
    pub ready: bool,
    pub status: String,
    pub status_label: String,
    pub reason: Option<String>,
    pub node_type: Option<String>,
    pub node_bound: Option<bool>,
    pub node_online: Option<bool>,
    pub account_id: Option<String>,
    pub expires_at_ms: Option<i64>,
    pub metadata: Option<CredentialSettings>,
    pub supported_use_cases: Vec<String>,
    pub readiness_by_use_case: HashMap<String, UseCaseReadiness>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CredentialSettings {
    pub region: Option<String>,
    pub credential_profile: Option<String>,
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UseCaseReadiness {
    pub ready: bool,
    pub model_count: usize,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub display_name: String,
    pub model_type: String,
    pub available: bool,
    pub deprecated: bool,
    pub successor: Option<String>,
    pub supports_tools: bool,
    pub supports_vision: bool,
    /// Missing catalog information stays unknown, never manufactured as zero.
    pub context_window_tokens: Option<u32>,
    pub input_modalities: Vec<String>,
    pub output_modalities: Vec<String>,
    pub chat_config: Option<ModelDefaults>,
    pub generation_controls: llm_api::GenerationControls,
    pub metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelDefaults {
    pub temperature_default: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelGroup {
    pub provider: String,
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultProvider {
    pub use_case: String,
    pub llm_mode: LlmMode,
    pub provider: String,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mapping {
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub reasoning_effort: Option<String>,
    #[serde(default)]
    pub thinking: Option<bool>,
    #[serde(default)]
    pub fast_mode: Option<bool>,
    pub use_case: String,
    pub agent_model_mode: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub configured_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRequirements {
    #[serde(default)]
    pub model_type: String,
    #[serde(default)]
    pub required_input_modalities: Vec<String>,
    #[serde(default)]
    pub optional_input_modalities: Vec<String>,
    #[serde(default)]
    pub required_output_modalities: Vec<String>,
    #[serde(default)]
    pub preferred_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UseCaseMode {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub requirements: ModelRequirements,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UseCase {
    #[serde(default)]
    pub use_case: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner_type: Option<String>,
    #[serde(default)]
    pub owner_id: Option<String>,
    #[serde(default)]
    pub execution_use_case: Option<String>,
    #[serde(default = "default_true")]
    pub configurable: bool,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub modes: Vec<UseCaseMode>,
    #[serde(default)]
    pub updated_at_ms: Option<i64>,
    #[serde(default)]
    pub last_seen_at_ms: Option<i64>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub llm_mode: LlmMode,
    pub providers: Vec<Provider>,
    pub provider_models: Vec<ModelGroup>,
    pub effective_mappings: Vec<Mapping>,
    pub default_providers: Vec<DefaultProvider>,
    pub llm_use_cases: Vec<UseCase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderList {
    /// Mode whose configuration was requested; reading it does not activate it.
    pub llm_mode: LlmMode,
    pub providers: Vec<Provider>,
    pub provider_models: Vec<ModelGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderDetail {
    pub llm_mode: LlmMode,
    pub provider: Provider,
    pub models: Vec<Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MappingList {
    pub llm_mode: LlmMode,
    pub effective_mappings: Vec<Mapping>,
    pub default_providers: Vec<DefaultProvider>,
    pub llm_use_cases: Vec<UseCase>,
}
