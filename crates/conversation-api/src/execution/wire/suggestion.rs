//! Non-conversation messages carried by the existing authenticated Agent link.
//! Payload documents are validated by the registered Suggestion input/output
//! models at each host. This leaf transport never depends on app-facade-api.
use super::{LlmExecutionPlan, ProtocolError};
use crate::execution::{
    ExternalError, FacadeRequest, OperationalContext, SuggestionInvocationContext, UsageSummary,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuggestionMessage {
    pub context: SuggestionInvocationContext,
    pub payload: SuggestionPayload,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum SuggestionPayload {
    /// Trusted admission snapshot; never decoded from a public generate input.
    Generate {
        input: Value,
        #[serde(deserialize_with = "deserialize_closed")]
        facts: OperationalContext,
        #[serde(deserialize_with = "deserialize_closed")]
        llm_plan: LlmExecutionPlan,
    },
    #[serde(deserialize_with = "crate::execution::deserialize_empty_variant")]
    Cancel,
    /// Only authorized reads and script verification are permitted by the host.
    Read { request: FacadeRequest },
    /// Verified typed generation output, not a model-authored completion status.
    Completed {
        result: Value,
        failures: Vec<SuggestionFailure>,
        #[serde(deserialize_with = "deserialize_closed")]
        usage: UsageSummary,
    },
    Failed {
        #[serde(deserialize_with = "deserialize_closed")]
        error: ExternalError,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuggestionFailure {
    pub candidate_index: Option<usize>,
    #[serde(deserialize_with = "deserialize_closed")]
    pub error: ExternalError,
}

// Internally tagged enums buffer their contents before deserializing fields, so
// the envelope's outer serde_ignored visitor cannot observe these nested maps.
fn deserialize_closed<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let mut unknown = None;
    let value = serde_ignored::deserialize(deserializer, |path| {
        unknown.get_or_insert_with(|| path.to_string());
    })?;
    match unknown {
        Some(path) => Err(serde::de::Error::custom(format!(
            "unknown protocol field: {path}"
        ))),
        None => Ok(value),
    }
}

impl SuggestionMessage {
    /// Validates transport shape, never the sender's authority or registered result type.
    /// # Errors
    /// Rejects malformed execution identity and non-object document payloads.
    pub fn validate(&self) -> Result<(), ProtocolError> {
        self.context
            .validate()
            .map_err(|e| ProtocolError::InvalidBinding(e.into()))?;
        match &self.payload {
            SuggestionPayload::Generate { input, .. } if !input.is_object() => Err(
                ProtocolError::InvalidBinding("Suggestion input must be an object".into()),
            ),
            SuggestionPayload::Completed { result, .. } if !result.is_object() => Err(
                ProtocolError::InvalidBinding("Suggestion result must be an object".into()),
            ),
            _ => Ok(()),
        }
    }
}
