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

/// Invocation status is independent of payload presence. Successful acknowledgements
/// omit data; partial outcomes always retain their result. Full data is a business
/// value and essential data is YAML text, selected by the request, not inferred here.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FacadeResponse {
    Success {
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
    Partial {
        data: Value,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
    Error {
        error: FacadeProblem,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<String>,
    },
    Cancelled {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        warnings: Vec<FacadeProblem>,
    },
}

impl FacadeResponse {
    /// A completed operation with no business value to return.
    pub fn acknowledged() -> Self {
        Self::Success {
            data: None,
            warnings: vec![],
        }
    }
    pub fn cancelled() -> Self {
        Self::Cancelled { warnings: vec![] }
    }
    pub fn success(data: Value) -> Self {
        Self::Success {
            data: Some(data),
            warnings: vec![],
        }
    }
    pub fn partial(data: Value) -> Self {
        Self::Partial {
            data,
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
            data: None,
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
            Self::Success { data, .. } => data.as_ref(),
            Self::Partial { data, .. } => Some(data),
            _ => None,
        }
    }
    pub fn data_mut(&mut self) -> Option<&mut Value> {
        match self {
            Self::Success { data, .. } => data.as_mut(),
            Self::Partial { data, .. } => Some(data),
            _ => None,
        }
    }
    /// Set the Essential presentation once, preserving the execution outcome.
    /// Full error details move into the projected diagnostic data. Acknowledgements
    /// and cancellations remain payload-free.
    pub fn with_essential_data(self, text: String) -> Self {
        match self {
            Self::Success {
                data: Some(_),
                warnings,
            } => Self::Success {
                data: Some(Value::String(text)),
                warnings,
            },
            Self::Partial { warnings, .. } => Self::Partial {
                data: Value::String(text),
                warnings,
            },
            Self::Error { mut error, .. } => {
                error.details = None;
                Self::Error {
                    error,
                    data: Some(text),
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
                let data = fields.remove("data");
                if status == "partial" && data.is_none() {
                    return Err(D::Error::custom("Partial outcome requires data"));
                }
                let warnings = fields
                    .remove("warnings")
                    .map(serde_json::from_value)
                    .transpose()
                    .map_err(D::Error::custom)?
                    .unwrap_or_default();
                if status == "success" {
                    Self::Success { data, warnings }
                } else {
                    Self::Partial {
                        data: data.expect("partial data checked"),
                        warnings,
                    }
                }
            }
            Some("error") => Self::Error {
                data: fields
                    .remove("data")
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
            json!({"status":"success","data":"items: []"}),
            json!({"status":"success"}),
            json!({"status":"success","warnings":[{"code":"SYNC_PENDING","message":"Saved"}]}),
            json!({"status":"partial","data":{"done":["a"],"failed":["b"]}}),
            json!({"status":"error","error":{"code":"STALE","message":"Changed"}}),
            json!({"status":"error","error":{"code":"BAD_REQUEST","message":"Invalid"},"data":"status: error\n"}),
            json!({"status":"cancelled"}),
        ] {
            let parsed: FacadeResponse = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        }
        for value in [
            json!({"status":"partial"}),
            json!({"status":"success","text":"legacy"}),
            json!({"status":"success","data":{},"text":"bad"}),
            json!({"status":"cancelled","data":{}}),
            json!({"status":"error","error":{"message":"bad"}}),
            json!({"status":"error","error":{"code":"BAD","message":"Invalid"},"data":{}}),
            json!({"status":"error","error":{"code":"BAD","message":"Invalid"},"text":"legacy"}),
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
            result.with_essential_data("state: failed".into()).status(),
            FacadeStatus::Success
        );
    }
}
