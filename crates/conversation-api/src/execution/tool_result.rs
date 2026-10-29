//! Provider-visible text for canonical tool results; presentation never changes history.
use super::ResponseDetail;
use serde_json::{Value, json};

/// Serialize an Essential projection, omitting null-valued object fields recursively.
/// Keep array positions, root null, empty collections, booleans, numbers and text intact.
/// This presentation rule never mutates the full business value or parses string content.
pub fn essential_yaml(value: &Value) -> Result<String, serde_yaml::Error> {
    fn omit_null_fields(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.retain(|_, value| !value.is_null());
                fields.values_mut().for_each(omit_null_fields);
            }
            Value::Array(items) => items.iter_mut().for_each(omit_null_fields),
            _ => {}
        }
    }
    let mut projected = value.clone();
    omit_null_fields(&mut projected);
    serde_yaml::to_string(&projected)
}

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
            return essential_yaml(&visible).expect("serializable result");
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
    fn essential_omits_null_fields_without_losing_values_or_array_positions() {
        let full = json!({"missing":null,"nested":{"missing":null,"enabled":false,
            "zero":0,"empty":"","list":[],"object":{},"literal":"null",
            "profile":"missing: null\nKeep this text\n"},"rows":[null,{"missing":null,"id":"a"},{}]});
        let before = full.clone();
        let parsed: Value = serde_yaml::from_str(&essential_yaml(&full).unwrap()).unwrap();
        assert_eq!(
            parsed,
            json!({"nested":{"enabled":false,"zero":0,"empty":"",
            "list":[],"object":{},"literal":"null","profile":"missing: null\nKeep this text\n"},
            "rows":[null,{"id":"a"},{}]})
        );
        assert_eq!(full, before);
        assert_eq!(essential_yaml(&Value::Null).unwrap(), "null\n");
    }

    #[test]
    fn late_warning_and_error_details_follow_the_same_null_rule() {
        let value = json!({"status":"error","error":{"code":"FAILED","message":"Failed",
            "details":{"missing":null,"clientReport":{"verified":false,"missing":null}}},
            "data":"status: error\n", "warnings":[{"code":"LATE","message":"Late warning",
            "details":{"missing":null,"retry":false}}]});
        let parsed: Value =
            serde_yaml::from_str(&tool_result_text(&value, ResponseDetail::Essential)).unwrap();
        assert_eq!(
            parsed["error"]["details"],
            json!({"clientReport":{"verified":false}})
        );
        assert_eq!(parsed["warnings"][0]["details"], json!({"retry":false}));
        assert!(value["error"]["details"].get("missing").is_some());
    }
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
