use app_facade_api::{
    authorization::{required, OperationPolicy},
    routing::{route_policy_for_target, ExecutionSelector},
};
use serde_json::{json, Value};

#[test]
fn client_appearance_report_is_scoped_and_cannot_choose_identity() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/client-appearance.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema["$defs"]["request"]).unwrap();
    let value = json!({"themeMode":"auto", "effectiveThemeMode":"dark", "primaryColor":"blue", "background":"forest", "cardOpacity":"medium", "cardOpacityApplied":true});
    assert!(validator.is_valid(&value));
    let mut spoof = value.clone();
    spoof["clientId"] = json!("a:other");
    assert!(!validator.is_valid(&spoof));
    let mut invalid = value;
    invalid["effectiveThemeMode"] = json!("auto");
    assert!(!validator.is_valid(&invalid));
    assert!(!validator.is_valid(&json!({"background":"forest"})));
    let target = "/app/client/appearance/report";
    assert_eq!(required(target), Some(OperationPolicy::SpaceRead));
    let route = route_policy_for_target(target).unwrap();
    assert_eq!(route.execution, ExecutionSelector::ScopeMode);
}
