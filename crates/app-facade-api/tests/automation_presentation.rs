use app_facade_api::{
    authorization::{required, OperationPolicy},
    automation_presentation::*,
};
use serde_json::json;

#[test]
fn names_schema_matches_bounded_request_contract() {
    let mut schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schema/automation-presentation.v1.schema.json"
    ))
    .unwrap();
    schema["$ref"] = json!("#/$defs/namesInput");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for value in [json!({"ids":["a"]}), json!({"ids":[" P-device ","a"]})] {
        assert!(validator.is_valid(&value), "{value}");
    }
    for value in [
        json!({"ids":[]}),
        json!({"ids":[" "]}),
        json!({"ids":vec!["a";101]}),
        json!({"ids":["a"],"includeScript":true}),
    ] {
        assert!(!validator.is_valid(&value), "{value}");
    }
}

#[test]
fn detail_essential_preserves_runnable_without_widening_group_fields() {
    let value = app_facade_api::response::essential(
        "/app/automation/detail/get",
        &json!({"automations":[{"id":"a","runnable":false,"script":{"script":"secret"}}, {"id":"g:a","group":true}]}),
    );
    assert_eq!(value["automations"][0]["runnable"], false);
    assert!(value["automations"][0].get("script").is_none());
    assert!(value["automations"][1].get("runnable").is_none());
}

#[test]
fn names_is_a_scoped_selected_read_and_retired_routes_have_no_admission() {
    let input: NamesInput =
        serde_json::from_value(json!({"ids":[" a ","a","P-device","missing"]})).unwrap();
    assert_eq!(
        input.normalized_ids().unwrap(),
        ["a", "P-device", "missing"]
    );
    for value in [
        json!({"ids":[]}),
        json!({"ids":[" "]}),
        json!({"ids":vec!["a";101]}),
    ] {
        assert!(serde_json::from_value::<NamesInput>(value)
            .unwrap()
            .normalized_ids()
            .is_err());
    }
    assert!(
        serde_json::from_value::<NamesInput>(json!({"ids":["a"],"includeScript":true})).is_err()
    );
    assert_eq!(required(NAMES_GET_TARGET), Some(OperationPolicy::SpaceRead));
    for target in [
        "/automation/batchGet",
        "/automation/delete",
        "/automation/names",
        "/automation/activity/list",
        "/automation/snapshot/get",
        "/automation/snapshot/batchDelete",
        "/ui/automationCard/create",
        "/automation/debug",
    ] {
        assert_eq!(required(target), None, "{target}");
    }
}
