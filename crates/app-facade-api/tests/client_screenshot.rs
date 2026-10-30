use app_facade_api::{
    authorization::{required, OperationPolicy},
    routing::{route_policy_for_target, ExecutionSelector},
};
use serde_json::{json, Value};
#[test]
fn support_report_is_scoped_strict_and_cannot_name_another_client() {
    let target = "/app/client/screenshot/report";
    assert_eq!(required(target), Some(OperationPolicy::SpaceRead));
    assert_eq!(
        route_policy_for_target(target).unwrap().execution,
        ExecutionSelector::ScopeMode
    );
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/client-screenshot.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/$defs/request");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for value in [true, false] {
        assert!(validator.is_valid(&json!({"supported":value})));
    }
    for input in [
        json!({}),
        json!({"supported":null}),
        json!({"supported":"true"}),
        json!({"supported":true,"clientId":"another"}),
    ] {
        assert!(!validator.is_valid(&input));
    }
}
