//! Public App result projection. Transport serialization is not a business input.
use crate::{FacadeResponse, ResponseDetail};
use conversation_api::execution::essential_yaml;
use serde_json::{json, Value};

mod agent;
mod runtime;
mod space;
pub mod time;
pub mod units;

/// Outcomes are assigned by target contracts, never by scanning arbitrary data.
pub fn outcome(target: &str, mut data: Value) -> FacadeResponse {
    if let Some(outcome) = space::outcome(target, &data) {
        return outcome;
    }
    if target == crate::group::DETAIL {
        let warnings: Vec<_> = data
            .get("items")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|item| item.get("group"))
            .filter(|group| !group.get("capabilities").is_some_and(Value::is_array))
            .map(|group| crate::FacadeProblem {
                code: "GROUP_MEMBERS_UNAVAILABLE".into(),
                message: "Group definition is available; member capabilities could not be read"
                    .into(),
                details: Some(json!({"groupId": group["groupId"]})),
            })
            .collect();
        return if warnings.is_empty() {
            FacadeResponse::success(data)
        } else {
            warnings
                .into_iter()
                .fold(FacadeResponse::partial(data), |response, warning| {
                    response.with_warning(warning)
                })
        };
    }
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
    if matches!(target, crate::automation::START | crate::automation::STOP) {
        return match serde_json::from_value::<crate::automation::ControlOutput>(data.clone()) {
            Ok(_) => FacadeResponse::success(data),
            Err(_) => {
                FacadeResponse::error("INVALID_RESPONSE", "Expected Automation control state")
            }
        };
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
    // The internal authorization DTO reports request completion separately from
    // the authorization session state. Only the latter is public business data.
    if matches!(
        target,
        "/app/agent/llm/provider/auth/start"
            | "/app/agent/llm/provider/auth/complete"
            | "/app/agent/llm/provider/auth/status"
    ) {
        match data.get("success").and_then(Value::as_bool) {
            Some(true) => {
                data.as_object_mut().unwrap().remove("success");
                return FacadeResponse::success(data);
            }
            Some(false) => {
                return FacadeResponse::error(
                    "OPERATION_FAILED",
                    "The authorization request did not succeed",
                )
                .with_details(data);
            }
            None => {
                return FacadeResponse::error(
                    "INVALID_RESPONSE",
                    "Expected an authorization request outcome",
                )
                .with_details(data);
            }
        }
    }
    // These mutation contracts use `success` as the operation outcome. Queries
    // (including execution-status reads) never use this test.
    let mutation = matches!(
        target,
        "/app/agent/identity/save"
            | "/app/agent/memory/save"
            | "/app/automation/save"
            | "/app/automation/description/update"
            | "/app/automation/delete"
            | "/app/automation/snapshot/rollback"
            | "/app/room/delete"
            | "/app/widget/delete"
            | "/app/widget/circadian/runtime/update"
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
        "/app/presentation/automation/generated/run"
            | "/app/presentation/automation/overview/cancel"
            | "/app/agent/identity/save"
            | "/app/agent/memory/save"
            | "/app/system/feedback/submit"
            | "/app/agent/mode/save"
            | "/app/agent/llm/mapping/save"
            | "/app/agent/llm/provider/credential/save"
            | "/app/agent/llm/provider/credential/delete"
            | "/app/agent/llm/provider/auth/cancel"
            | "/app/nfc/activate"
            | "/app/nfc/update"
            | "/app/nfc/delete"
            | "/app/widget/delete"
            | "/app/widget/circadian/runtime/update"
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
    } else if matches!(target, "/app/entity/list" | "/app/entity/detail/get") {
        entity_metadata(target, data)
    } else if target == "/app/widget/list" {
        widget_list(data)
    } else if target == "/app/widget/detail/get" {
        widget_definition(data)
    } else if target == "/app/device/list" {
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

// Directory metadata is shared by list and selected detail. Source-defined data
// remains intact: a provider-local data.id is not the canonical entity ID.
fn entity_metadata(target: &str, data: &Value) -> Value {
    let mut result = data.clone();
    let collection = if target == "/app/entity/list" {
        "entities"
    } else {
        "items"
    };
    if let Some(rows) = result.get_mut(collection).and_then(Value::as_array_mut) {
        for row in rows {
            let entity = if collection == "items" {
                row.get_mut("entity")
            } else {
                Some(row)
            };
            let Some(fields) = entity.and_then(Value::as_object_mut) else {
                continue;
            };
            fields.remove("tags");
            if fields.get("group").and_then(Value::as_bool) != Some(true) {
                fields.remove("entityIds");
            }
            for field in ["integration", "availabilityStatus"] {
                if fields.get(field).is_some_and(Value::is_null) {
                    fields.remove(field);
                }
            }
            let kind = fields
                .get("entityType")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_owned();
            if let Some(data) = fields.get_mut("data").and_then(Value::as_object_mut) {
                if kind == "Location" {
                    data.retain(|key, _| {
                        matches!(
                            key.as_str(),
                            "address"
                                | "timezone"
                                | "countryCode"
                                | "latitude"
                                | "longitude"
                                | "altitude"
                        )
                    });
                } else if kind == "AppClient" {
                    data.retain(|key, _| key == "clientType");
                }
            }
        }
    }
    result
}

// Omit only absent optional definition metadata, never runtime values or user objects.
fn widget_list(data: &Value) -> Value {
    let mut result = data.clone();
    if let Some(widgets) = result.get_mut("widgets").and_then(Value::as_array_mut) {
        for widget in widgets {
            if let Some(fields) = widget.as_object_mut() {
                for key in ["displayName", "description", "tag", "dataType", "repeated"] {
                    if fields.get(key).is_some_and(Value::is_null) {
                        fields.remove(key);
                    }
                }
            }
        }
    }
    result
}

// Keep editable configuration and identity; UI rendering helpers are not model context.
fn widget_definition(data: &Value) -> Value {
    let mut result = data.clone();
    if let Some(items) = result.get_mut("items").and_then(Value::as_array_mut) {
        for item in items {
            if let Some(widget) = item.get_mut("widget").and_then(Value::as_object_mut) {
                for field in ["global", "baseId", "steps", "stepUnits"] {
                    widget.remove(field);
                }
                for field in [
                    "description",
                    "tag",
                    "dataType",
                    "repeated",
                    "options",
                    "min",
                    "max",
                    "step",
                    "label",
                    "location",
                    "schedules",
                    "curve",
                    "offset",
                    "multiplier",
                ] {
                    if widget.get(field).is_some_and(Value::is_null) {
                        widget.remove(field);
                    }
                }
            }
        }
    }
    result
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
                            | "applyState"
                            | "roomId"
                            | "roomName"
                            | "capabilities"
                            | "group"
                            | "integration"
                            | "parent"
                            | "setupStatus"
                            | "availabilityStatus"
                            | "blueprintIds"
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
                "entityIds",
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
                "runnable",
                "singleUse",
                "hasExpire",
                "usedCapabilities",
                "group",
                "entityIds",
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
    render_with_units(target, response, detail, timezone, &Default::default())
}

pub fn render_with_units(
    target: &str,
    mut response: FacadeResponse,
    detail: ResponseDetail,
    timezone: &str,
    preferences: &std::collections::BTreeMap<String, String>,
) -> Result<FacadeResponse, serde_yaml::Error> {
    if detail == ResponseDetail::Essential {
        let warnings = match &mut response {
            FacadeResponse::Success { warnings, .. }
            | FacadeResponse::Partial { warnings, .. }
            | FacadeResponse::Cancelled { warnings } => Some(warnings),
            FacadeResponse::Error { .. } => None,
        };
        if let Some(warnings) = warnings {
            for warning in warnings {
                if let Some(details) = &mut warning.details {
                    units::localize(details, preferences);
                    time::localize_typed(details, timezone);
                }
            }
        }
    }
    if detail == ResponseDetail::Essential {
        if let FacadeResponse::Error { error, data: None } = &response {
            let mut problem = serde_json::to_value(error).expect("serializable error");
            if let Some(details) = problem.get_mut("details") {
                project_error_details(target, details);
            }
            units::localize(&mut problem, preferences);
            time::localize_typed(&mut problem, timezone);
            let text = essential_yaml(&json!({"status":"error","error":problem}))?;
            return Ok(response.with_essential_data(text));
        }
    }
    if detail == ResponseDetail::Essential && response.data().is_some() {
        let mut projected = essential(target, response.data().expect("outcome has data"));
        units::localize(&mut projected, preferences);
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
    fn device_discovery_keeps_parent_without_exposing_private_identity() {
        let rows = device_briefs(&json!({"devices":[{
            "entityId":"d1", "integration":"mlinkShelly.device",
            "parent":"shelly://aabbccddeeff", "setupStatus":"failed", "connectId":"node1",
            "manufacturerId":"shelly", "password":"secret"
        }, {"entityId":"g:1", "group":true}]}));
        assert_eq!(rows["devices"][0]["parent"], "shelly://aabbccddeeff");
        assert_eq!(rows["devices"][0]["setupStatus"], "failed");
        assert!(rows["devices"][0].get("connectId").is_none());
        assert!(rows["devices"][0].get("manufacturerId").is_none());
        assert!(rows["devices"][0].get("password").is_none());
        assert!(rows["devices"][1].get("parent").is_none());
    }
    #[test]
    fn widget_display_name_survives_full_and_essential_without_touching_object_fields() {
        let widget = json!({"id":"w-d","type":"data","displayName":"目标温度","dataType":"Person",
            "options":[{"uid":"person/a","displayName":"Alice","name":"schema-owned field"}]});
        let data = json!({"items":[{"requestedId":"w-d","found":true,"widget":widget}]});
        for detail in [ResponseDetail::Full, ResponseDetail::Essential] {
            let response = serde_json::to_value(
                present("/app/widget/detail/get", data.clone(), detail).unwrap(),
            )
            .unwrap();
            let visible = if detail == ResponseDetail::Essential {
                serde_yaml::from_str::<Value>(response["data"].as_str().unwrap()).unwrap()
            } else {
                response["data"].clone()
            };
            let actual = &visible["items"][0]["widget"];
            assert_eq!(actual["displayName"], "目标温度");
            assert!(actual.get("name").is_none());
            assert_eq!(actual["options"][0]["name"], "schema-owned field");
        }
    }

    #[test]
    fn plugin_business_fields_do_not_determine_outcome() {
        for target in [
            "/app/plugin/audiobridge/device/settings/get",
            "/app/plugin/llm/model/delete",
            "/app/plugin/matter/inventory/get",
        ] {
            let data = json!({"data":{"ok":false,"status":"error","reason":"resource state"}});
            assert_eq!(outcome(target, data.clone()), FacadeResponse::success(data));
            for detail in [ResponseDetail::Full, ResponseDetail::Essential] {
                let result = render(
                    target,
                    FacadeResponse::error("PLUGIN_BUSY", "Try later")
                        .with_details(json!({"retryAfter":2})),
                    detail,
                )
                .unwrap();
                let value = serde_json::to_value(result).unwrap();
                assert_eq!(value["status"], "error");
                assert_eq!(value["error"]["code"], "PLUGIN_BUSY");
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
        assert!(!outcome(
            "/app/plugin/llm/model/delete",
            json!({"data":{"ok":true,"deleted":true}})
        )
        .is_error());
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
