//! Public App result projection. Transport serialization is not a business input.
use crate::{FacadeResponse, ResponseDetail};
use conversation_api::execution::essential_yaml;
use serde_json::{json, Value};

mod agent;
mod runtime;
pub mod time;

/// Outcomes are assigned by target contracts, never by scanning arbitrary data.
pub fn outcome(target: &str, data: Value) -> FacadeResponse {
    if matches!(
        target,
        crate::group::CREATE
            | crate::group::RENAME
            | crate::group::MEMBERS_UPDATE
            | crate::group::DELETE
    ) {
        if let Some(problem) = data.get("applicationProblem").filter(|p| !p.is_null()) {
            return match serde_json::from_value::<crate::FacadeProblem>(problem.clone()) {
                Ok(problem) => FacadeResponse::partial(data).with_warning(problem),
                Err(_) => {
                    FacadeResponse::error("INVALID_RESPONSE", "Invalid Group application outcome")
                }
            };
        }
        return FacadeResponse::success(data);
    }
    if is_acknowledgement(target) {
        return match data.get("success").and_then(Value::as_bool) {
            Some(true) => FacadeResponse::acknowledged(),
            Some(false) => FacadeResponse::error(
                "OPERATION_FAILED",
                "The requested operation did not succeed",
            )
            .with_details(data),
            None => {
                FacadeResponse::error("INVALID_RESPONSE", "Expected an operation acknowledgement")
                    .with_details(data)
            }
        };
    }
    // These mutation contracts use `success` as the operation outcome. Queries
    // (including execution-status reads) never use this test.
    let mutation = matches!(
        target,
        "/app/agent/identity/save"
            | "/app/agent/memory/save"
            | "/app/automation/save"
            | "/app/automation/description/update"
            | "/app/automation/start"
            | "/app/automation/stop"
            | "/app/automation/delete"
            | "/app/automation/snapshot/rollback"
            | "/app/room/delete"
            | "/app/widget/delete"
            | "/app/widget/data/update"
            | "/app/widget/button/trigger"
            | "/app/integration/install"
            | "/app/integration/uninstall"
            | "/app/integration/disconnect"
    );
    if mutation && data.get("success").and_then(Value::as_bool) == Some(false) {
        let message = data
            .get("message")
            .or_else(|| data.get("error"))
            .and_then(Value::as_str)
            .unwrap_or("The requested operation did not succeed")
            .to_owned();
        return FacadeResponse::error("OPERATION_FAILED", message).with_details(data);
    }
    if target.starts_with("/app/plugin/camera/")
        && data
            .get("ok")
            .or_else(|| data.get("data").and_then(|payload| payload.get("ok")))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return FacadeResponse::error(
            "PLUGIN_OPERATION_FAILED",
            "Camera operation did not succeed",
        )
        .with_details(data);
    }
    if target.starts_with("/app/plugin/audiobridge/")
        && data.pointer("/data/ok").and_then(Value::as_bool) == Some(false)
    {
        return FacadeResponse::error(
            data.pointer("/data/reason")
                .and_then(Value::as_str)
                .unwrap_or("PLUGIN_OPERATION_FAILED"),
            data.pointer("/data/detail")
                .and_then(Value::as_str)
                .unwrap_or("AudioBridge operation did not succeed"),
        )
        .with_details(data);
    }
    // LLM mutation outcomes live inside the PluginNodeResponse envelope.
    // Runtime readiness and installation-state reads remain resource data.
    if matches!(
        target,
        "/app/plugin/llm/model/install"
            | "/app/plugin/llm/model/install/cancel"
            | "/app/plugin/llm/model/delete"
            | "/app/plugin/llm/model/import/start"
            | "/app/plugin/llm/model/import/finalize"
            | "/app/plugin/llm/model/import/cancel"
    ) && data.pointer("/data/ok").and_then(Value::as_bool) == Some(false)
    {
        let code = data
            .pointer("/data/reason")
            .and_then(Value::as_str)
            .unwrap_or("PLUGIN_OPERATION_FAILED");
        let message = data
            .pointer("/data/detail")
            .and_then(Value::as_str)
            .unwrap_or("Local AI operation did not succeed");
        return FacadeResponse::error(code, message).with_details(data);
    }
    let incomplete = match target {
        "/app/device/enable" | "/app/device/disable" => nonempty(&data, "pendingIds"),
        "/app/room/devices/move" => nonempty(&data, "notMoved"),
        "/app/room/devices/remove" => nonempty(&data, "notRemoved"),
        "/app/capability/invoke" | "/app/capability/invoke/json" => nonempty(&data, "errors"),
        _ => false,
    };
    if incomplete {
        FacadeResponse::partial(data)
    } else {
        FacadeResponse::success(data)
    }
}
/// Routes whose completed mutations return no business value. Reads never use this rule.
pub fn is_acknowledgement(target: &str) -> bool {
    matches!(
        target,
        "/app/agent/identity/save"
            | "/app/agent/memory/save"
            | "/app/system/feedback/submit"
            | "/app/agent/mode/save"
            | "/app/agent/llm/mapping/save"
            | "/app/agent/llm/provider/credential/save"
            | "/app/agent/llm/provider/credential/delete"
            | "/app/automation/start"
            | "/app/automation/stop"
            | "/app/widget/delete"
            | "/app/widget/data/update"
            | "/app/widget/button/trigger"
    )
}
fn nonempty(data: &Value, key: &str) -> bool {
    data.get(key)
        .and_then(Value::as_array)
        .is_some_and(|v| !v.is_empty())
}

/// Select public fields for each operation before rendering the model-facing result.
pub fn essential(target: &str, data: &Value) -> Value {
    if let Some(result) = agent::project(target, data) {
        result
    } else if let Some(result) = runtime::project(target, data) {
        result
    } else if target == "/app/device/brief/list" {
        device_briefs(data)
    } else if target.starts_with("/app/dashboard/") {
        dashboard(
            data,
            !matches!(
                target,
                "/app/dashboard/card/detail/get" | "/app/dashboard/card/list"
            ),
        )
    } else {
        automation(target, data)
    }
}

// Discovery keeps functional and availability metadata. Connection and manufacturer
// identity belongs to selected detail reads; never filter state/history payloads.
fn device_briefs(data: &Value) -> Value {
    let mut result = data.clone();
    if let Some(devices) = result.get_mut("devices").and_then(Value::as_array_mut) {
        for device in devices {
            if let Some(fields) = device.as_object_mut() {
                fields.retain(|key, _| {
                    matches!(
                        key.as_str(),
                        "entityId"
                            | "name"
                            | "active"
                            | "roomId"
                            | "roomName"
                            | "capabilities"
                            | "group"
                            | "integration"
                            | "online"
                            | "availabilityStatus"
                            | "twinTags"
                    )
                });
            }
        }
    }
    result
}

// Project only read DTO rows. Diagnostic detail retains its bounded evidence and logs.
fn automation(target: &str, data: &Value) -> Value {
    let (collection, fields): (&str, &[&str]) = match target {
        "/app/automation/list" => (
            "automations",
            &[
                "id",
                "name",
                "description",
                "automationType",
                "enabled",
                "singleUse",
                "group",
                "memberIds",
            ],
        ),
        "/app/automation/detail/get" => (
            "automations",
            &[
                "id",
                "name",
                "description",
                "automationType",
                "enabled",
                "singleUse",
                "hasExpire",
                "usedCapabilities",
                "group",
                "memberIds",
                "inputVars",
                "scriptHash",
                "descriptionTemplate",
            ],
        ),
        "/app/automation/activity/now/list" => (
            "automations",
            &[
                "automationId",
                "automationName",
                "automationType",
                "singleUse",
            ],
        ),
        "/app/automation/activity/recent/list" => (
            "activities",
            &[
                "activityId",
                "automationId",
                "automationName",
                "automationType",
                "singleUse",
                "activityType",
                "timestamp",
                "instanceId",
                "threadId",
            ],
        ),
        "/app/automation/snapshot/list" => (
            "snapshots",
            &["snapshotId", "createdAt", "description", "automationType"],
        ),
        _ => return data.clone(),
    };
    let mut result = data.clone();
    if let Some(items) = result.get_mut(collection).and_then(Value::as_array_mut) {
        for item in items {
            if let Some(object) = item.as_object_mut() {
                object.retain(|key, _| fields.contains(&key.as_str()));
            }
        }
    }
    result
}

fn dashboard(value: &Value, include_layout: bool) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|value| dashboard(value, include_layout))
                .collect(),
        ),
        Value::Object(fields) if fields.contains_key("cardId") && fields.contains_key("type") => {
            let mut result = serde_json::Map::new();
            for key in ["cardId", "type", "title", "layouts", "primaryViewId"] {
                if key == "layouts" && !include_layout {
                    continue;
                }
                if let Some(v) = fields.get(key) {
                    result.insert(key.into(), v.clone());
                }
            }
            let views = fields
                .get("views")
                .and_then(Value::as_array)
                .map(|views| {
                    views
                        .iter()
                        .map(|v| {
                            let mut item = serde_json::Map::new();
                            for key in ["id", "name"] {
                                if let Some(value) = v.get(key) {
                                    item.insert(key.into(), value.clone());
                                }
                            }
                            Value::Object(item)
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if fields.contains_key("views") {
                result.insert("viewCount".into(), json!(views.len()));
                result.insert("views".into(), Value::Array(views));
            }
            Value::Object(result)
        }
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(k, v)| (k.clone(), dashboard(v, include_layout)))
                .collect(),
        ),
        value => value.clone(),
    }
}

pub fn present(
    target: &str,
    data: Value,
    detail: ResponseDetail,
) -> Result<FacadeResponse, serde_yaml::Error> {
    render(target, outcome(target, data), detail)
}

/// Render a settled outcome without changing its status or executing it again.
pub fn render(
    target: &str,
    response: FacadeResponse,
    detail: ResponseDetail,
) -> Result<FacadeResponse, serde_yaml::Error> {
    render_in_timezone(target, response, detail, "UTC")
}

pub fn present_in_timezone(
    target: &str,
    data: Value,
    detail: ResponseDetail,
    timezone: &str,
) -> Result<FacadeResponse, serde_yaml::Error> {
    render_in_timezone(target, outcome(target, data), detail, timezone)
}

pub fn render_in_timezone(
    target: &str,
    response: FacadeResponse,
    detail: ResponseDetail,
    timezone: &str,
) -> Result<FacadeResponse, serde_yaml::Error> {
    if detail == ResponseDetail::Essential {
        if let FacadeResponse::Error { error, data: None } = &response {
            let mut problem = serde_json::to_value(error).expect("serializable error");
            if let Some(details) = problem.get_mut("details") {
                project_error_details(target, details);
            }
            let text = essential_yaml(&json!({"status":"error","error":problem}))?;
            return Ok(response.with_essential_data(text));
        }
    }
    if detail == ResponseDetail::Essential && response.data().is_some() {
        let mut projected = essential(target, response.data().expect("outcome has data"));
        time::localize(target, &mut projected, timezone);
        let text = essential_yaml(&projected)?;
        Ok(response.with_essential_data(text))
    } else {
        Ok(response)
    }
}

/// Only this domain owns its diagnostic projection; unrelated error details remain intact.
fn project_error_details(target: &str, details: &mut Value) {
    if target != "/app/automation/description/update" {
        return;
    }
    if let Some(diagnostics) = details.get_mut("diagnostics").and_then(Value::as_array_mut) {
        let mut unique = Vec::new();
        for item in diagnostics.iter() {
            let mut projected = serde_json::Map::new();
            for field in ["field", "code"] {
                if let Some(value) = item.get(field).filter(|v| !v.is_null()) {
                    projected.insert(field.into(), value.clone());
                }
            }
            if let Some(message) = item
                .get("summary")
                .filter(|v| !v.is_null())
                .or_else(|| item.get("description").filter(|v| !v.is_null()))
            {
                projected.insert("message".into(), message.clone());
            }
            let projected = Value::Object(projected);
            if !unique.contains(&projected) {
                unique.push(projected);
            }
        }
        *diagnostics = unique;
    }
}

// The protocol owns visible tool text; domain projections remain in this crate.
pub use conversation_api::execution::tool_result_text as tool_text;

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn audio_failures_preserve_the_reason_but_runtime_failure_is_state() {
        let failure = outcome(
            "/app/plugin/audiobridge/device/settings/get",
            json!({"data":{"ok":false,"reason":"AUDIOBRIDGE_OPERATION_FAILED","detail":"Device is not paired"}}),
        );
        let value = serde_json::to_value(failure).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"]["message"], "Device is not paired");
        let state = outcome(
            "/app/plugin/audiobridge/device/snapshot",
            json!({"data":{"devices":[{"listenRuntime":{"state":"failed"}}]}}),
        );
        assert_eq!(serde_json::to_value(state).unwrap()["status"], "success");
    }

    #[test]
    fn llm_mutation_failures_preserve_details_in_full_and_essential_results() {
        let data = json!({"nodeId":"llm-1","nodeType":"llm","version":"v1",
            "data":{"ok":false,"reason":"MODEL_BUSY","detail":"Model installation is busy"}});
        for route in [
            "install",
            "install/cancel",
            "delete",
            "import/start",
            "import/finalize",
            "import/cancel",
        ] {
            for detail in [ResponseDetail::Full, ResponseDetail::Essential] {
                let result = present(
                    &format!("/app/plugin/llm/model/{route}"),
                    data.clone(),
                    detail,
                )
                .unwrap();
                assert!(result.is_error());
                let value = serde_json::to_value(result).unwrap();
                assert_eq!(value["error"]["code"], "MODEL_BUSY");
                if detail == ResponseDetail::Full {
                    assert_eq!(value["error"]["details"], data);
                } else {
                    let yaml: Value =
                        serde_yaml::from_str(value["data"].as_str().unwrap()).unwrap();
                    assert_eq!(yaml["error"]["details"], data);
                }
            }
        }
    }

    #[test]
    fn llm_queries_do_not_treat_resource_failure_as_invocation_failure() {
        let data = json!({"nodeId":"llm-1","data":{"ok":false,"ready":false,"operation":{"state":"error"}}});
        for route in [
            "status",
            "model/list",
            "model/install/status",
            "model/import/status",
            "engine/status",
            "engine/operation",
        ] {
            assert!(!outcome(&format!("/app/plugin/llm/{route}"), data.clone()).is_error());
        }
        assert!(
            !outcome(
                "/app/plugin/llm/model/delete",
                json!({"data":{"ok":true,"deleted":true}})
            )
            .is_error()
        );
    }
    #[test]
    fn dashboard_card_reads_omit_layout_but_full_is_unchanged() {
        let full = json!({"items":[{"cardId":"c1","status":"found","card":{"cardId":"c1","type":"ENTITY","title":"Lights","layouts":{"desktop":{"x":0,"y":0}},"views":[{"id":"v1","name":"Switch","source":{"secret":"internal"},"pattern":{}}],"primaryViewId":"v1","widgets":[{"value":100}],"entity":{"capabilities":[]}}}]});
        let result = essential("/app/dashboard/card/detail/get", &full);
        assert_eq!(result["items"][0]["card"]["viewCount"], 1);
        assert_eq!(
            result["items"][0]["card"]["views"],
            json!([{"id":"v1","name":"Switch"}])
        );
        assert!(result["items"][0]["card"].get("widgets").is_none());
        assert!(result["items"][0]["card"].get("layouts").is_none());
        let list = essential(
            "/app/dashboard/card/list",
            &json!({"cards": [full["items"][0]["card"].clone()]}),
        );
        assert!(list["cards"][0].get("layouts").is_none());
        assert_eq!(full["items"][0]["card"]["widgets"][0]["value"], 100);
    }
    #[test]
    fn partial_preserves_failed_ids_and_query_state_does_not_change_outcome() {
        let data = json!({"moved":["a"],"notMoved":[{"deviceId":"b","reason":"offline"}]});
        assert!(outcome("/app/room/devices/move", data.clone()).is_error());
        assert!(!outcome("/app/room/list", data).is_error());
    }
    #[test]
    fn replay_presentation_preserves_canonical_result_and_warnings() {
        let cached = FacadeResponse::partial(json!({"moved":["a"],"notMoved":[{"deviceId":"b"}]}))
            .with_warning(crate::FacadeProblem {
                code: "OFFLINE".into(),
                message: "Retry later".into(),
                details: None,
            });
        let full = render(
            "/app/room/devices/move",
            cached.clone(),
            ResponseDetail::Full,
        )
        .unwrap();
        let essential = render(
            "/app/room/devices/move",
            cached.clone(),
            ResponseDetail::Essential,
        )
        .unwrap();
        assert_eq!(full, cached);
        let text = serde_json::to_value(essential).unwrap();
        assert_eq!(text["status"], "partial");
        assert_eq!(text["warnings"][0]["code"], "OFFLINE");
        assert_eq!(
            serde_yaml::from_str::<Value>(text["data"].as_str().unwrap()).unwrap(),
            *cached.data().unwrap()
        );
    }
    #[test]
    fn error_essential_is_actionable_yaml_and_keeps_full_untouched() {
        let issue = json!({"field":"description","code":"variable_not_defined",
            "description":"Original full parser explanation and template", "summary":"Input $missing is not declared",
            "start":{"row":2,"col":18},"end":{"row":2,"col":26},"level":"error"});
        let full = FacadeResponse::error("BAD_REQUEST", "Validation failed").with_details(json!({
            "automationId":"a","reason":"validation_failed","applied":false,"diagnostics":[issue.clone(),issue]}));
        let original = serde_json::to_value(&full).unwrap();
        let rendered = render(
            "/app/automation/description/update",
            full.clone(),
            ResponseDetail::Essential,
        )
        .unwrap();
        assert!(rendered.is_error());
        let value = serde_json::to_value(rendered).unwrap();
        assert!(value["error"].get("details").is_none());
        let text = tool_text(&value, ResponseDetail::Essential);
        let visible: Value = serde_yaml::from_str(&text).unwrap();
        assert_eq!(visible["status"], "error");
        assert_eq!(visible["error"]["details"]["applied"], false);
        assert_eq!(
            visible["error"]["details"]["diagnostics"],
            json!([
            {"field":"description","code":"variable_not_defined","message":"Input $missing is not declared"}])
        );
        assert!(!text.contains("start:"));
        assert!(!text.contains("Original full"));
        assert_eq!(
            serde_json::to_value(
                render(
                    "/app/automation/description/update",
                    full,
                    ResponseDetail::Full
                )
                .unwrap()
            )
            .unwrap(),
            original
        );
        let decoded: FacadeResponse = serde_json::from_value(value).unwrap();
        assert!(decoded.is_error());
    }
}
