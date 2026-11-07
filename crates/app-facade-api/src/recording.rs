//! Household behavior projections. Pure and independent of persistence/presentation.
//! Each registered Facade owns an explicit input/output projection; all others skip.
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::sync::LazyLock;

static SPEC: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../manifest/recording.v1.json")).expect("recording contract")
});

pub fn selected(target: &str) -> bool {
    descriptor(target).is_some()
}
fn descriptor(target: &str) -> Option<&'static Value> {
    SPEC["operations"]
        .as_array()?
        .iter()
        .find(|d| d["target"] == target)
}

/// A projector may skip a mixed-purpose Facade. Never persist a raw call or response.
pub fn project(target: &str, input: &Value, outcome: &crate::FacadeResponse) -> Option<Value> {
    let d = descriptor(target)?;
    let data = serde_json::to_value(outcome).ok()?;
    let mut body = Map::new();
    body.insert("event".into(), d["event"].clone());
    match d["hook"].as_str() {
        Some("capability") => capability(input, &mut body)?,
        Some("automationCopy") => {
            select(&mut body, input, &d["input"]);
            if let Some(id) = input.get("automationId") {
                body.insert("fromAutomationId".into(), id.clone());
            }
        }
        Some("automationSource") => {
            select(&mut body, input, &d["input"]);
            if let Some(text) = input.pointer("/script/script").and_then(Value::as_str) {
                if let Ok(source) = serde_yaml::from_str::<Value>(text) {
                    let mut summary = Map::new();
                    select(&mut summary, &source, &json!(["name", "description"]));
                    if !summary.is_empty() {
                        body.insert("definition".into(), Value::Object(summary));
                    }
                }
            }
        }
        Some("circadianDelete") => {
            let ids: Vec<_> = input["widgetIds"]
                .as_array()?
                .iter()
                .filter(|id| id.as_str().is_some_and(|s| s.ends_with("-c")))
                .cloned()
                .collect();
            if ids.is_empty() {
                return None;
            }
            body.insert("widgetIds".into(), Value::Array(ids));
        }
        hook => {
            if hook == Some("circadian")
                && target.ends_with("/update")
                && ![
                    "schedules",
                    "curve",
                    "offset",
                    "multiplier",
                    "locationId",
                    "location_id",
                ]
                .iter()
                .any(|key| input.get(key).is_some())
            {
                return None;
            }
            select(&mut body, input, &d["input"]);
        }
    }
    select(&mut body, &data["data"], &d["output"]);
    body.insert("status".into(), data["status"].clone());
    // Codes are enough for learning failed intent. Internal error details/messages stay out.
    if let Some(code) = data.pointer("/error/code") {
        body.insert("errorCode".into(), code.clone());
    }
    Some(Value::Object(body))
}

fn select(out: &mut Map<String, Value>, source: &Value, fields: &Value) {
    for field in fields
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        let value = field.split('.').try_fold(source, |v, part| v.get(part));
        // Preserve false, zero and explicit null (e.g. resetting a value).
        if let Some(value) = value {
            out.insert(field.replace('.', "_"), value.clone());
        }
    }
    if !out.contains_key("widgetId") {
        if let Some(id) = source.get("widget_id").or_else(|| source.get("id")) {
            if fields
                .as_array()
                .is_some_and(|f| f.iter().any(|v| v == "widgetId"))
            {
                out.insert("widgetId".into(), id.clone());
            }
        }
    }
    if !out.contains_key("locationId")
        && fields
            .as_array()
            .is_some_and(|f| f.iter().any(|v| v == "locationId"))
    {
        if let Some(id) = source.get("location_id") {
            out.insert("locationId".into(), id.clone());
        }
    }
}
fn capability(input: &Value, out: &mut Map<String, Value>) -> Option<()> {
    let id = input["capabilityId"].as_str()?;
    let name = SPEC["deviceCapabilities"].get(id)?.as_str()?;
    let fields = SPEC["deviceCommands"].get(name)?;
    let request: Value = match &input["request"] {
        Value::String(text) => serde_yaml::from_str(text).ok()?,
        value => value.clone(),
    };
    if name == "setCameraSetting" {
        // Generic camera settings may also contain transport/codec configuration.
        if request.get("monitoringMode").is_none() && request.get("personRecognitionMode").is_none()
        {
            return None;
        }
    }
    out.insert("capabilityId".into(), json!(id));
    let mut values = Map::new();
    select(&mut values, &request, fields);
    if let Some(device) = values.get_mut("device").filter(|v| v.is_object()) {
        let mut reference = Map::new();
        select(&mut reference, device, &json!(["uid", "name"]));
        *device = Value::Object(reference);
    }
    out.insert("parameters".into(), Value::Object(values));
    Some(())
}

/// Saved business configuration: resolved household parameters, without executable source.
/// Invoked by a Facade-owned background hook, including editor saves whose public input is a session ID.
pub fn automation_facts(yaml: &str) -> Option<Value> {
    let source: Value = serde_yaml::from_str(yaml).ok()?;
    let object = source.as_object()?;
    let mut definition = Map::new();
    select(&mut definition, &source, &json!(["name", "description"]));
    let parameters: Map<String, Value> = object
        .iter()
        .filter(|(key, _)| key.starts_with('$'))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if !parameters.is_empty() {
        definition.insert("parameters".into(), Value::Object(parameters));
    }
    if let Some(input_vars) = object.get("inputVars").and_then(Value::as_array) {
        let variables: Vec<_> = input_vars
            .iter()
            .filter_map(|variable| {
                let mut reference = Map::new();
                select(
                    &mut reference,
                    variable,
                    &json!(["name", "type", "description"]),
                );
                (!reference.is_empty()).then_some(Value::Object(reference))
            })
            .collect();
        if !variables.is_empty() {
            definition.insert("inputVariables".into(), json!(variables));
        }
    }
    (!definition.is_empty()).then_some(json!({"definition":definition}))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BehaviorRecord {
    pub id: String,
    pub occurred_at_ms: i64,
    pub finished_at_ms: i64,
    pub actor: Value,
    pub source: Value,
    pub data: Value,
}

/// Fixed-width UTC milliseconds preserve numeric order as UTF-8 strings.
pub fn time_key(timestamp_ms: i64) -> Result<String, &'static str> {
    if !(0..=9_999_999_999_999).contains(&timestamp_ms) {
        return Err("timestamp outside 13 digit range");
    }
    Ok(format!("{timestamp_ms:013}|"))
}
pub fn sort_key(timestamp_ms: i64, id: &str) -> Result<String, &'static str> {
    if id.is_empty() {
        return Err("invalid record id");
    }
    Ok(format!("{}{id}", time_key(timestamp_ms)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_projection_corpus() {
        let cases: Value =
            serde_json::from_str(include_str!("../fixtures/recording.conformance.json")).unwrap();
        for case in cases.as_array().unwrap() {
            let result: crate::FacadeResponse =
                serde_json::from_value(case["outcome"].clone()).unwrap();
            assert_eq!(
                project(case["target"].as_str().unwrap(), &case["input"], &result)
                    .unwrap_or(Value::Null),
                case["expected"],
                "{}",
                case["target"]
            );
        }
    }
    #[test]
    fn every_registered_facade_has_a_projection_case() {
        let cases: Value =
            serde_json::from_str(include_str!("../fixtures/recording.conformance.json")).unwrap();
        for operation in SPEC["operations"].as_array().unwrap() {
            assert!(
                cases
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|c| c["target"] == operation["target"]),
                "{}",
                operation["target"]
            );
        }
    }
    #[test]
    fn saved_configuration_facts_keep_values_without_source() {
        let cases: Value = serde_json::from_str(include_str!(
            "../fixtures/recording-automation.conformance.json"
        ))
        .unwrap();
        for case in cases.as_array().unwrap() {
            assert_eq!(
                automation_facts(case["source"].as_str().unwrap()).unwrap_or(Value::Null),
                case["expected"]
            );
        }
    }
    #[test]
    fn projections_are_selected_and_compact() {
        let input = json!({"widgetId":"x-d", "typedValue":0,"secret":"hidden"});
        let full = crate::response::outcome(
            "/app/widget/data/update",
            json!({"success":true,"internal":"hidden"}),
        );
        let data = project("/app/widget/data/update", &input, &full).unwrap();
        assert_eq!(data["typedValue"], 0);
        assert_eq!(data["status"], "success");
        assert!(!data.to_string().contains("hidden"));
        assert!(project("/app/device/disable", &input, &full).is_none());
        assert!(project("/app/plugin/camera/device/settings/update", &input, &full).is_none());
        assert!(project("/app/widget/delete", &json!({"widgetIds":["x-d"]}), &full).is_none());
    }
    #[test]
    fn time_range_is_half_open_and_ids_do_not_collide() {
        let a = sort_key(99, "a").unwrap();
        let b = sort_key(100, "b").unwrap();
        assert!(a < b);
        assert!(a >= time_key(99).unwrap());
        assert!(b > time_key(100).unwrap());
        assert_ne!(sort_key(99, "a"), sort_key(99, "b"));
    }
    #[test]
    fn yaml_and_json_commands_match_and_queries_skip() {
        let result = crate::FacadeResponse::acknowledged();
        let a=project("/app/capability/invoke",&json!({"capabilityId":"basic.device.command.onOff","request":"device: lamp\ncommand: 'off'"}),&result).unwrap();
        let b=project("/app/capability/invoke/json",&json!({"capabilityId":"basic.device.command.onOff","request":{"device":"lamp","command":"off"}}),&result).unwrap();
        assert_eq!(a, b);
        assert!(project(
            "/app/capability/invoke/json",
            &json!({"capabilityId":"basic.device.command.getImageSnapshot","request":{}}),
            &result
        )
        .is_none());
    }
}
