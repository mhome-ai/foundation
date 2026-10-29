//! Provider-visible text for canonical tool results; presentation never changes history.
use super::ResponseDetail;
use serde_json::{Value, json};

/// Text at model/MCP/CLI boundaries. Canonical history keeps the original outcome.
/// Full responses and non-Facade values retain JSON; Essential content is never JSON-escaped.
pub fn tool_result_text(value: &Value, detail: ResponseDetail) -> String {
    if let Some(text) = (detail == ResponseDetail::Essential)
        .then(|| value.get("data").and_then(Value::as_str))
        .flatten()
    {
        if matches!(
            value.get("status").and_then(Value::as_str),
            Some("success" | "partial" | "error")
        ) {
            let warnings = value
                .get("warnings")
                .filter(|v| v.as_array().is_some_and(|v| !v.is_empty()));
            let extra_error_details =
                value["status"] == "error" && value["error"].get("details").is_some();
            if value["status"] != "partial" && warnings.is_none() && !extra_error_details {
                return text.to_owned();
            }
            let result =
                serde_yaml::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.into()));
            let mut visible = json!({"status":value["status"],"result":result});
            if extra_error_details {
                visible["error"] = value["error"].clone();
            }
            if let Some(warnings) = warnings {
                visible["warnings"] = warnings.clone();
            }
            return serde_yaml::to_string(&visible).expect("serializable result");
        }
    }
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn visible_text_preserves_partial_outcomes_and_warnings() {
        let full_string = json!({"status":"success","data":"items: []\n"});
        assert_eq!(
            tool_result_text(&full_string, ResponseDetail::Full),
            full_string.to_string()
        );
        assert_eq!(
            tool_result_text(&json!({"status":"success"}), ResponseDetail::Essential),
            r#"{"status":"success"}"#
        );
        for status in ["success", "partial"] {
            let value = json!({"status":status,"data":"items: []\n", "warnings":[{"code":"UNAVAILABLE","message":"One source is offline"}]});
            let text = tool_result_text(&value, ResponseDetail::Essential);
            let yaml: Value = serde_yaml::from_str(&text).unwrap();
            assert_eq!(yaml["status"], status);
            assert_eq!(yaml["warnings"], value["warnings"]);
            assert_eq!(yaml["result"], json!({"items":[]}));
        }
        assert_eq!(
            tool_result_text(
                &json!({"status":"success","data":"items: []\n"}),
                ResponseDetail::Essential
            ),
            "items: []\n"
        );
        assert_eq!(
            tool_result_text(
                &json!({"data":{"status":"error"}}),
                ResponseDetail::Essential
            ),
            r#"{"data":{"status":"error"}}"#
        );
    }
    #[test]
    fn late_error_observations_survive_visible_output() {
        let value = json!({"status":"error", "error":{"code":"FAILED","message":"Failed", "details":{"clientReport":{"verified":false}}}, "data":"status: error\nerror:\n  code: FAILED\n  message: Failed\n"});
        let visible: Value =
            serde_yaml::from_str(&tool_result_text(&value, ResponseDetail::Essential)).unwrap();
        assert_eq!(visible["error"]["details"], value["error"]["details"]);
        assert_eq!(visible["result"]["status"], "error");
    }
}
