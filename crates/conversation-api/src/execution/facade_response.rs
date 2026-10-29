//! Invocation outcomes, independent of the state of the resource being queried.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseDetail {
    #[default]
    Full,
    Essential,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacadeStatus {
    Success,
    Partial,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FacadeProblem {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Content {
    Data { data: Value },
    Text { text: String },
}

/// The public response has exactly one content representation on success/partial,
/// an error with optional Essential text on failure, and no content on cancellation. Constructors and strict
/// deserialization enforce this invariant on both sides of the wire.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FacadeResponse {
    Success {
        #[serde(flatten)]
        content: Content,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
    Partial {
        #[serde(flatten)]
        content: Content,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
    Error {
        error: FacadeProblem,
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
    Cancelled {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
}

impl FacadeResponse {
    pub fn cancelled() -> Self {
        Self::Cancelled { warnings: vec![] }
    }
    pub fn success(data: Value) -> Self {
        Self::Success {
            content: Content::Data { data },
            warnings: vec![],
        }
    }
    pub fn partial(data: Value) -> Self {
        Self::Partial {
            content: Content::Data { data },
            warnings: vec![],
        }
    }
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            error: FacadeProblem {
                code: code.into(),
                message: message.into(),
                details: None,
            },
            text: None,
        }
    }
    pub fn with_details(mut self, details: Value) -> Self {
        if let Self::Error { error, .. } = &mut self {
            error.details = Some(details);
        }
        self
    }
    pub fn status(&self) -> FacadeStatus {
        match self {
            Self::Success { .. } => FacadeStatus::Success,
            Self::Partial { .. } => FacadeStatus::Partial,
            Self::Error { .. } => FacadeStatus::Error,
            Self::Cancelled { .. } => FacadeStatus::Cancelled,
        }
    }
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Partial { .. } | Self::Error { .. })
    }
    pub fn data(&self) -> Option<&Value> {
        match self {
            Self::Success {
                content: Content::Data { data },
                ..
            }
            | Self::Partial {
                content: Content::Data { data },
                ..
            } => Some(data),
            _ => None,
        }
    }
    pub fn data_mut(&mut self) -> Option<&mut Value> {
        match self {
            Self::Success {
                content: Content::Data { data },
                ..
            }
            | Self::Partial {
                content: Content::Data { data },
                ..
            } => Some(data),
            _ => None,
        }
    }
    /// Presentation never changes execution outcome or discards warnings.
    pub fn with_text(self, text: String) -> Self {
        match self {
            Self::Success { warnings, .. } => Self::Success {
                content: Content::Text { text },
                warnings,
            },
            Self::Partial { warnings, .. } => Self::Partial {
                content: Content::Text { text },
                warnings,
            },
            Self::Error { mut error, .. } => {
                error.details = None;
                Self::Error {
                    error,
                    text: Some(text),
                }
            }
            terminal => terminal,
        }
    }
    pub fn with_warning(mut self, warning: FacadeProblem) -> Self {
        match &mut self {
            Self::Success { warnings, .. }
            | Self::Partial { warnings, .. }
            | Self::Cancelled { warnings } => warnings.push(warning),
            _ => {}
        }
        self
    }
}

impl<'de> Deserialize<'de> for FacadeResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = Value::deserialize(deserializer)?;
        let mut fields = value
            .as_object()
            .cloned()
            .ok_or_else(|| D::Error::custom("Facade response must be an object"))?;
        let status = fields
            .remove("status")
            .ok_or_else(|| D::Error::custom("Missing Facade status"))?;
        let response = match status.as_str() {
            Some("success" | "partial") => {
                let content = match (fields.remove("data"), fields.remove("text")) {
                    (Some(data), None) => Content::Data { data },
                    (None, Some(Value::String(text))) => Content::Text { text },
                    _ => return Err(D::Error::custom("Exactly one of data or text is required")),
                };
                let warnings = fields
                    .remove("warnings")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_default();
                if status == "success" {
                    Self::Success { content, warnings }
                } else {
                    Self::Partial { content, warnings }
                }
            }
            Some("error") => Self::Error {
                text: fields
                    .remove("text")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(D::Error::custom)?,
                error: serde_json::from_value(
                    fields
                        .remove("error")
                        .ok_or_else(|| D::Error::custom("Missing Facade error"))?,
                )
                .map_err(D::Error::custom)?,
            },
            Some("cancelled") => Self::Cancelled {
                warnings: fields
                    .remove("warnings")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_default(),
            },
            _ => return Err(D::Error::custom("Unknown Facade status")),
        };
        if !fields.is_empty() {
            return Err(D::Error::custom("Unknown Facade response field"));
        }
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn strict_response_roundtrip() {
        for value in [
            json!({"status":"success","data":null}),
            json!({"status":"success","text":"items: []"}),
            json!({"status":"partial","data":{"done":["a"],"failed":["b"]}}),
            json!({"status":"error","error":{"code":"STALE","message":"Changed"}}),
            json!({"status":"cancelled"}),
        ] {
            let parsed: FacadeResponse = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        }
        for value in [
            json!({"status":"success"}),
            json!({"status":"success","data":{},"text":"bad"}),
            json!({"status":"cancelled","data":{}}),
            json!({"status":"error","error":{"message":"bad"}}),
            json!({"status":"done","data":{}}),
        ] {
            assert!(serde_json::from_value::<FacadeResponse>(value).is_err());
        }
    }
    #[test]
    fn resource_failure_is_not_invocation_failure() {
        let result = FacadeResponse::success(json!({"status":"failed","success":false}));
        assert!(!result.is_error());
        assert_eq!(
            result.with_text("state: failed".into()).status(),
            FacadeStatus::Success
        );
    }
    #[test]
    fn error_text_roundtrips_and_stays_an_error() {
        let value = serde_json::json!({"status":"error","error":{"code":"BAD_REQUEST","message":"Invalid"},"text":"status: error\n"});
        let response: FacadeResponse = serde_json::from_value(value.clone()).unwrap();
        assert!(response.is_error());
        assert_eq!(serde_json::to_value(response).unwrap(), value);
    }
}
