use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Provider-issued replay data attached to exactly one completed model message.
/// Adapters own the format and compatibility key. Callers must not synthesize or edit payloads.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Continuation {
    pub format: String,
    pub compatibility_key: String,
    pub payload: Value,
}

impl std::fmt::Debug for Continuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Continuation")
            .field("format", &self.format)
            .field("compatibility_key", &self.compatibility_key)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opaque_state_roundtrips_but_is_redacted_from_debug() {
        let c = Continuation {
            format: "openrouter.reasoning.v1".into(),
            compatibility_key: "model".into(),
            payload: serde_json::json!({"reasoning_details":[{"signature":"secret-signature"}]}),
        };
        assert_eq!(
            serde_json::from_slice::<Continuation>(&serde_json::to_vec(&c).unwrap()).unwrap(),
            c
        );
        assert!(!format!("{c:?}").contains("secret-signature"));
    }
}
