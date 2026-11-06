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
        "/automation/list",
        "/automation/info",
        "/automation/card/get",
        "/automation/batchGet",
        "/automation/delete",
        "/automation/names",
        "/automation/activity/list",
        "/automation/snapshot/get",
        "/automation/snapshot/batchDelete",
        "/ui/automationCard/create",
        "/automation/debug",
        "/automation/get",
        "/automation/status",
        "/automation/enable",
        "/automation/disable",
        "/automation/run",
        "/automation/copy",
        "/automation/enroll",
        "/automation/share",
        "/automation/format",
        "/automation/save",
        "/automation/overview/init",
        "/automation/overview/updateConfig",
        "/automation/overview/provideConfig",
        "/automation/overview/save",
        "/automation/overview/cancel",
    ] {
        assert_eq!(required(target), None, "{target}");
    }
}

#[test]
fn current_page_inputs_reject_unbounded_or_unrelated_expansion() {
    for value in [json!({"type":"RECURRING"}), json!({"type":"TASK_FLOW"})] {
        serde_json::from_value::<ListInput>(value).unwrap();
    }
    for value in [
        json!({}),
        json!({"type":"GROUP"}),
        json!({"type":1}),
        json!({"type":"RECURRING","cycle":"temp"}),
    ] {
        assert!(serde_json::from_value::<ListInput>(value).is_err());
    }
    let mut schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schema/automation-presentation.v1.schema.json"
    ))
    .unwrap();
    schema["$ref"] = json!("#/$defs/listInput");
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({"type":"RECURRING"})));
    assert!(!validator.is_valid(&json!({"type":1})));
    assert!(!validator.is_valid(&json!({"type":"RECURRING","includeScript":true})));
    for target in [LIST_TARGET, OVERVIEW_GET_TARGET, PLACEMENTS_GET_TARGET] {
        assert_eq!(required(target), Some(OperationPolicy::SpaceRead));
    }
    assert!(
        serde_json::from_value::<AutomationInput>(json!({"automationId":" "}))
            .unwrap()
            .normalized_id()
            .is_err()
    );
    assert!(serde_json::from_value::<AutomationInput>(
        json!({"automationId":"a","includeSnapshots":true})
    )
    .is_err());
}

#[test]
fn list_time_presentation_localizes_explicit_fields_without_changing_full_output() {
    let data = json!({"automations":[{"id":"a","timestamp":1793088060000i64,"createdAt":1793088060000i64,"updatedAt":1793088060000i64,"enabledAt":1793088060000i64}]});
    let full = serde_json::to_value(
        app_facade_api::response::present_in_timezone(
            LIST_TARGET,
            data.clone(),
            app_facade_api::ResponseDetail::Full,
            "Asia/Shanghai",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(full["data"], data);
    let essential = serde_json::to_value(
        app_facade_api::response::present_in_timezone(
            LIST_TARGET,
            data,
            app_facade_api::ResponseDetail::Essential,
            "Asia/Shanghai",
        )
        .unwrap(),
    )
    .unwrap();
    assert!(essential["data"]
        .as_str()
        .unwrap()
        .contains("Asia/Shanghai"));
    assert!(essential["data"]
        .as_str()
        .unwrap()
        .contains("2026-10-27 16:01:00 +08:00"));
}

#[test]
fn ui_mutations_have_explicit_authority_and_generated_execution_is_closed() {
    assert_eq!(required(SUMMARIES_GET), Some(OperationPolicy::SpaceRead));
    for target in [
        EDITOR_INIT,
        EDITOR_CONFIG_UPDATE,
        EDITOR_CONFIG_PROVIDE,
        EDITOR_SAVE,
        EDITOR_CANCEL,
        COPY,
        ENROLL,
        SHARE,
        GENERATED_ENABLE,
        GENERATED_DISABLE,
        GENERATED_RUN,
    ] {
        assert_eq!(
            required(target),
            Some(OperationPolicy::SpaceWrite),
            "{target}"
        );
    }
    for id in ["P-device", " S-sync ", "G-group"] {
        assert_eq!(generated_program_id(id).unwrap(), id.trim());
    }
    for id in ["a", "T-once", "g:group", "P-", " "] {
        assert!(generated_program_id(id).is_err(), "{id}");
    }
    for field in ["sideData", "userId", "clientId"] {
        let mut input = json!({"automationId":"P-device"});
        input[field] = json!({});
        assert!(serde_json::from_value::<GeneratedRunInput>(input).is_err());
    }
    for target in [GENERATED_RUN, EDITOR_CANCEL] {
        let result = serde_json::to_value(
            app_facade_api::response::present(
                target,
                json!({"success":true}),
                app_facade_api::ResponseDetail::Full,
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(result, json!({"status":"success"}));
    }
}

#[test]
fn selected_summary_times_and_saved_projection_match_the_schema() {
    let value = json!({"items":[{"requestedId":"a","found":true,"resource":{"id":"a","automationId":"a","name":"Light","type":"RECURRING","enabled":true,"corrupted":false,"runnable":true,"running":false,"blocked":false,"hasExpire":false,"timestamp":1793088060000i64,"createdAt":0,"updatedAt":0,"enabledAt":0,"tags":[],"temp":false,"lastRunAt":1793088060000i64,"expireAt":null}},{"requestedId":"missing","found":false}]});
    let parsed: SummariesOutput = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(parsed.items.len(), 2);
    let mut schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schema/automation-presentation.v1.schema.json"
    ))
    .unwrap();
    schema["$ref"] = json!("#/$defs/summariesOutput");
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&value));
    let shown = serde_json::to_value(
        app_facade_api::response::present_in_timezone(
            SUMMARIES_GET,
            value,
            app_facade_api::ResponseDetail::Essential,
            "Asia/Shanghai",
        )
        .unwrap(),
    )
    .unwrap();
    let text = shown["data"].as_str().unwrap();
    assert!(text.contains("2026-10-27 16:01:00 +08:00"), "{text}");
    schema["$ref"] = json!("#/$defs/savedOutput");
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({"automationId":"copy","runnable":true,"enabled":false})));
    assert!(!validator.is_valid(
        &json!({"automationId":"copy","runnable":true,"enabled":false,"script":"secret"})
    ));
}
