//! Current display state of the receiving authenticated Client connection.
use crate::{ActionMode, FacadeCall, ResponseDetail};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const FOCUS_TARGET: &str = "/app/client/space/focus";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FocusInput {
    pub target_space_id: Option<String>,
}

impl FocusInput {
    pub fn parse(payload: &str) -> Result<(Self, ResponseDetail), &'static str> {
        let call: FacadeCall =
            serde_json::from_str(payload).map_err(|_| "Expected an App Facade call")?;
        call.validate()?;
        if call.control.mode != ActionMode::Direct {
            return Err("Focus requires direct mode");
        }
        let object = call
            .input
            .as_object()
            .ok_or("Focus input must be an object")?;
        if object.len() != 1 || !object.contains_key("targetSpaceId") {
            return Err("Focus requires targetSpaceId (text or null)");
        }
        let input: Self =
            serde_json::from_value(call.input).map_err(|_| "Invalid targetSpaceId")?;
        if input
            .target_space_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty() || id.trim() != id)
        {
            return Err("targetSpaceId must be non-empty unpadded text or null");
        }
        Ok((
            input,
            call.control.response_detail.unwrap_or(ResponseDetail::Full),
        ))
    }
}

pub fn focus_call(space_id: Option<&str>) -> String {
    serde_json::to_string(&FacadeCall::direct(json!({"targetSpaceId":space_id})))
        .expect("serializable focus call")
}

pub fn focus_data(space_id: Option<&str>) -> Value {
    json!({"focusedSpaceId":space_id})
}

/// Native switching commits local selection only after a canonical successful result.
pub fn require_success(payload: &str) -> Result<(), &'static str> {
    let value: Value = serde_json::from_str(payload).map_err(|_| "Invalid Focus response")?;
    if value.get("status").and_then(Value::as_str) != Some("success") || value.get("data").is_none()
    {
        return Err("Focus did not complete successfully");
    }
    Ok(())
}
