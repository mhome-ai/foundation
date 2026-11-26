//! Complete Suggestion results, separate from generation and business execution.
//!
//! The generic envelope does not assume a particular output kind. Producers must
//! select a registered content model and validate it before persistence. A model
//! supplies a draft; the backend owns identity, version, lifecycle and outcomes.

use crate::{FacadeResponse, FacadeStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub const GENERATE: &str = "/app/suggestion/generate";
pub const GENERATION_GET: &str = "/app/suggestion/generation/get";
pub const LIST: &str = "/app/suggestion/list";
pub const DETAIL_GET: &str = "/app/suggestion/detail/get";
pub const HANDLE: &str = "/app/suggestion/handle";
pub const FEEDBACK: &str = "/app/suggestion/feedback";
pub const SETTINGS_GET: &str = "/app/suggestion/settings/get";
pub const SETTINGS_SAVE: &str = "/app/suggestion/settings/save";
pub const TARGETS: &[&str] = &[
    GENERATE,
    GENERATION_GET,
    LIST,
    DETAIL_GET,
    HANDLE,
    FEEDBACK,
    SETTINGS_GET,
    SETTINGS_SAVE,
];

pub const DEFAULT_SUGGESTION_INTERVAL_MINUTES: u32 = 360;

const fn default_interval_minutes() -> u32 {
    DEFAULT_SUGGESTION_INTERVAL_MINUTES
}

/// Space-owned periodic generation policy. The interval is measured between
/// automatic start times; explicit generation does not change that clock.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionSettings {
    pub enabled: bool,
    #[serde(default = "default_interval_minutes")]
    pub interval_minutes: u32,
}

impl Default for SuggestionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: default_interval_minutes(),
        }
    }
}

impl SuggestionSettings {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.interval_minutes == 0 {
            return Err("Suggestion intervalMinutes must be a positive integer");
        }
        Ok(())
    }

    pub fn interval_ms(&self) -> u64 {
        u64::from(self.interval_minutes) * 60_000
    }
}

/// Update only supplied fields, at the Space backend. Null is not a reset value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveSettingsInput {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub enabled: Option<bool>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub interval_minutes: Option<u32>,
}

fn present<'de, T: Deserialize<'de>, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

impl SaveSettingsInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.enabled.is_none() && self.interval_minutes.is_none() {
            return Err("Provide enabled or intervalMinutes");
        }
        if self.interval_minutes == Some(0) {
            return Err("Suggestion intervalMinutes must be a positive integer");
        }
        Ok(())
    }

    pub fn apply_to(
        &self,
        mut settings: SuggestionSettings,
    ) -> Result<SuggestionSettings, &'static str> {
        self.validate()?;
        if let Some(enabled) = self.enabled {
            settings.enabled = enabled;
        }
        if let Some(minutes) = self.interval_minutes {
            settings.interval_minutes = minutes;
        }
        settings.validate()?;
        Ok(settings)
    }
}

/// A concrete excerpt, not a model confidence score or a substitute for evidence.
/// The producer checks references against authorized reads; consumers do not
/// interpret missing/truncated records as evidence that an event did not occur.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Evidence {
    pub source_type: String,
    pub source_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at: Option<u64>,
    pub excerpt: String,
}

pub trait SuggestionContent: Serialize {
    fn suggestion_type(&self) -> &str;
    fn validate(&self) -> Result<(), &'static str>;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionDraft<C> {
    pub title: String,
    pub summary: String,
    pub reasons: Vec<String>,
    pub evidence: Vec<Evidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
    #[serde(flatten)]
    pub content: C,
}

impl<C: SuggestionContent> SuggestionDraft<C> {
    pub fn validate(&self) -> Result<(), &'static str> {
        if blank(&self.title)
            || blank(&self.summary)
            || self.reasons.is_empty()
            || self.reasons.iter().any(|v| blank(v))
            || self.evidence.is_empty()
        {
            return Err("A Suggestion requires a title, summary, reasons and evidence");
        }
        if !upper_name(self.content.suggestion_type()) {
            return Err("Suggestion type must use UPPER_CASE");
        }
        for evidence in &self.evidence {
            if !upper_name(&evidence.source_type)
                || blank(&evidence.source_id)
                || blank(&evidence.excerpt)
                || evidence.occurred_at.is_some_and(|v| !timestamp(v))
            {
                return Err("Invalid Suggestion evidence");
            }
        }
        if self.expires_at.is_some_and(|v| !timestamp(v)) {
            return Err("Invalid Suggestion expiration timestamp");
        }
        // An extension cannot shadow backend metadata or shared explanation fields.
        let content =
            serde_json::to_value(&self.content).map_err(|_| "Invalid Suggestion content")?;
        let object = content
            .as_object()
            .ok_or("Suggestion content must be an object")?;
        if object.get("type").and_then(Value::as_str) != Some(self.content.suggestion_type())
            || object
                .keys()
                .any(|key| RESERVED_FIELDS.contains(&key.as_str()))
        {
            return Err("Suggestion content conflicts with the common envelope");
        }
        self.content.validate()
    }
}

const RESERVED_FIELDS: &[&str] = &[
    "id",
    "spaceId",
    "runId",
    "version",
    "status",
    "createdAt",
    "updatedAt",
    "title",
    "summary",
    "reasons",
    "evidence",
    "expiresAt",
    "resolution",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuggestionStatus {
    Pending,
    Resolved,
    Dismissed,
    Expired,
}

/// Resolution is supplied by the business handler, not by SuggestionGenerator.
/// RESOLVED means the one-shot handling has ended, not that every business step
/// succeeded. Preserve its canonical full result, including partial/error outcomes;
/// this is a receipt, not a recovery task. Legacy completed receipts may omit it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionResolution {
    pub action: String,
    pub operation_id: String,
    pub handled_at: u64,
    pub actor: conversation_api::ClientPrincipal,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resource_ids: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<FacadeResponse>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion<C> {
    pub id: String,
    pub space_id: String,
    pub run_id: String,
    pub version: u64,
    pub status: SuggestionStatus,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<SuggestionResolution>,
    #[serde(flatten)]
    pub draft: SuggestionDraft<C>,
}

impl<C: SuggestionContent> Suggestion<C> {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.draft.validate()?;
        if [&self.id, &self.space_id, &self.run_id]
            .iter()
            .any(|v| blank(v))
            || self.version == 0
            || self.version > MAX_SAFE_INTEGER
            || !timestamp(self.created_at)
            || !timestamp(self.updated_at)
            || self.updated_at < self.created_at
            || self.draft.expires_at.is_some_and(|v| v <= self.created_at)
        {
            return Err("Invalid Suggestion metadata");
        }
        let needs_resolution = matches!(
            self.status,
            SuggestionStatus::Resolved | SuggestionStatus::Dismissed
        );
        if needs_resolution != self.resolution.is_some() {
            return Err("Suggestion lifecycle requires a matching business resolution");
        }
        if let Some(resolution) = &self.resolution {
            if blank(&resolution.action)
                || blank(&resolution.operation_id)
                || match &resolution.actor {
                    conversation_api::ClientPrincipal::User { user_id } => blank(user_id),
                    conversation_api::ClientPrincipal::Space {
                        tenant_id,
                        space_id,
                    } => blank(tenant_id) || space_id != &self.space_id,
                }
                || resolution.handled_at < self.created_at
                || resolution.handled_at > self.updated_at
                || resolution
                    .resource_ids
                    .iter()
                    .any(|(key, value)| blank(key) || blank(value))
                || resolution.feedback.as_ref().is_some_and(|v| blank(v))
            {
                return Err("Invalid Suggestion resolution");
            }
        }
        Ok(())
    }
}

/// Complete existing AutomationScript source shape. This is not an executable
/// binary: the ordinary Automation service remains responsible for compilation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AutomationScript {
    pub script: String,
    #[serde(default)]
    pub scripts: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parse_results: Option<Vec<Value>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AutomationOperation {
    Create,
    Update,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum AutomationContent {
    Automation {
        operation: AutomationOperation,
        #[serde(
            default,
            rename = "automationId",
            skip_serializing_if = "Option::is_none"
        )]
        automation_id: Option<String>,
        #[serde(
            default,
            rename = "baseScriptHash",
            skip_serializing_if = "Option::is_none"
        )]
        base_script_hash: Option<String>,
        script: AutomationScript,
    },
}

impl SuggestionContent for AutomationContent {
    fn suggestion_type(&self) -> &str {
        "AUTOMATION"
    }
    fn validate(&self) -> Result<(), &'static str> {
        let Self::Automation {
            operation,
            automation_id,
            base_script_hash,
            script,
        } = self;
        match operation {
            AutomationOperation::Create
                if automation_id.is_some() || base_script_hash.is_some() =>
            {
                return Err("CREATE cannot identify an existing Automation")
            }
            AutomationOperation::Update
                if automation_id.as_ref().is_none_or(|v| blank(v))
                    || base_script_hash.as_ref().is_none_or(|v| blank(v)) =>
            {
                return Err("UPDATE requires automationId and baseScriptHash")
            }
            _ => (),
        }
        if blank(&script.script)
            || script.scripts.keys().any(|name| blank(name))
            || script.sdk_version.as_ref().is_some_and(|v| blank(v))
        {
            return Err("A complete Automation script source is required");
        }
        // Source validation/compilation is a separate, required runtime operation.
        Ok(())
    }
}

pub type AutomationSuggestion = Suggestion<AutomationContent>;

/// Portable generator output before backend identity/lifecycle assignment. An
/// empty successful result is distinct from an execution error. The host owns
/// run status and measured usage; model text cannot set either of them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionGenerationResult<C> {
    pub suggestions: Vec<SuggestionDraft<C>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_summary: Option<String>,
}

impl<C: SuggestionContent> SuggestionGenerationResult<C> {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.context_summary.as_ref().is_some_and(|v| blank(v)) {
            return Err("contextSummary must be nonempty when supplied");
        }
        self.suggestions
            .iter()
            .try_for_each(SuggestionDraft::validate)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub suggestion_type: String,
    pub version: u64,
    pub title: String,
    pub summary: String,
    pub status: SuggestionStatus,
    /// Outcome of the completed handling, independent of lifecycle status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<FacadeStatus>,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<u64>,
}

impl<C: SuggestionContent> Suggestion<C> {
    pub fn summary(&self) -> SuggestionSummary {
        SuggestionSummary {
            id: self.id.clone(),
            suggestion_type: self.draft.content.suggestion_type().into(),
            version: self.version,
            title: self.draft.title.clone(),
            summary: self.draft.summary.clone(),
            status: self.status,
            outcome: self
                .resolution
                .as_ref()
                .and_then(|r| r.result.as_ref())
                .map(FacadeResponse::status),
            created_at: self.created_at,
            updated_at: self.updated_at,
            expires_at: self.draft.expires_at,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

impl GenerateInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.focus.as_ref().is_some_and(|v| blank(v)) {
            return Err("focus must be nonempty when supplied");
        }
        Ok(())
    }
}

/// Admission acknowledgement. When another run is active, accepted is false:
/// that run is returned, but this request's focus and payer are not substituted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerateOutput {
    pub accepted: bool,
    pub run: SuggestionRun,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationGetInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

impl GenerationGetInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.run_id.as_ref().is_some_and(|v| blank(v)) {
            return Err("runId must be nonempty when supplied");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GenerationGetOutput {
    /// Current or most recent run; null before any run. An explicit unknown or
    /// no-longer-retained run ID is an ordinary Facade NOT_FOUND error.
    #[serde(deserialize_with = "Option::deserialize")]
    pub run: Option<SuggestionRun>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListInput {
    #[serde(default)]
    pub types: Vec<String>,
    #[serde(default)]
    pub statuses: Vec<SuggestionStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u16,
}

fn default_limit() -> u16 {
    50
}

impl ListInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=100).contains(&self.limit)
            || self.types.iter().any(|v| !upper_name(v))
            || self.cursor.as_ref().is_some_and(|v| blank(v))
        {
            return Err("Invalid Suggestion list query");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ListOutput {
    pub items: Vec<SuggestionSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DetailInput {
    pub suggestion_id: String,
}

impl DetailInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if blank(&self.suggestion_id) {
            return Err("suggestionId is required");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HandleInput {
    pub suggestion_id: String,
    pub expected_version: u64,
    /// Must be an action declared by this Suggestion type; never a Facade target.
    pub action: String,
}

impl HandleInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if blank(&self.suggestion_id)
            || self.expected_version == 0
            || self.expected_version > MAX_SAFE_INTEGER
            || !upper_name(&self.action)
        {
            return Err(
                "A Suggestion action requires identity, version and a declared action name",
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FeedbackInput {
    pub suggestion_id: String,
    pub expected_version: u64,
    pub decision: FeedbackDecision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl FeedbackInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if blank(&self.suggestion_id)
            || self.expected_version == 0
            || self.expected_version > MAX_SAFE_INTEGER
            || self.comment.as_ref().is_some_and(|v| blank(v))
        {
            return Err("Invalid Suggestion feedback");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FeedbackDecision {
    Dismiss,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SuggestionRunStatus {
    Running,
    Succeeded,
    Partial,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SuggestionRun {
    pub run_id: String,
    pub status: SuggestionRunStatus,
    pub started_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<u64>,
    pub suggestion_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

impl SuggestionRun {
    pub fn validate(&self) -> Result<(), &'static str> {
        if blank(&self.run_id)
            || !timestamp(self.started_at)
            || self.suggestion_ids.iter().any(|v| blank(v))
            || self
                .finished_at
                .is_some_and(|v| !timestamp(v) || v < self.started_at)
            || self.error_code.as_ref().is_some_and(|v| blank(v))
        {
            return Err("Invalid Suggestion run metadata");
        }
        let running = self.status == SuggestionRunStatus::Running;
        if running == self.finished_at.is_some() {
            return Err("Run completion timestamp does not match its status");
        }
        if matches!(
            self.status,
            SuggestionRunStatus::Partial | SuggestionRunStatus::Failed
        ) && self.error_code.is_none()
            || matches!(
                self.status,
                SuggestionRunStatus::Running | SuggestionRunStatus::Succeeded
            ) && self.error_code.is_some()
        {
            return Err("Run status does not match its error");
        }
        if matches!(
            self.status,
            SuggestionRunStatus::Running
                | SuggestionRunStatus::Failed
                | SuggestionRunStatus::Cancelled
        ) && !self.suggestion_ids.is_empty()
        {
            return Err("Uncompleted runs cannot claim published Suggestion results");
        }
        Ok(())
    }
}

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
fn timestamp(value: u64) -> bool {
    value > 0 && value <= MAX_SAFE_INTEGER
}
fn blank(value: &str) -> bool {
    value.trim().is_empty()
}
fn upper_name(value: &str) -> bool {
    value.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
        && value
            .bytes()
            .all(|v| v.is_ascii_uppercase() || v.is_ascii_digit() || v == b'_')
}
