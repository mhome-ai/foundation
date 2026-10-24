use app_facade_api::{
    authorization::{required, OperationPolicy},
    routing::{route_policy_for_target, ExecutionSelector},
};
use serde_json::{json, Value};
#[test]
fn navigation_snapshot_contract_is_scoped_and_excludes_caller_identity() {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/client-page.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/$defs/request");
    let validator = jsonschema::validator_for(&schema).unwrap();
    let page = json!({"current":{"page":"dashboard","presentation":"page"}});
    assert!(validator.is_valid(&page));
    let mut drawer = json!({"current":{"page":"entity.detail","presentation":"drawer","payload":{"uid":"one","entityType":"basic.device.Device"}}});
    assert!(!validator.is_valid(&drawer));
    drawer["base"] = page["current"].clone();
    assert!(validator.is_valid(&drawer));
    drawer["base"]["presentation"] = json!("drawer");
    assert!(!validator.is_valid(&drawer));
    let mut spoof = page.clone();
    spoof["clientId"] = json!("other");
    assert!(!validator.is_valid(&spoof));
    let mut invalid = page;
    invalid["current"]["payload"] = json!([]);
    assert!(!validator.is_valid(&invalid));
    let target = "/app/client/page/report";
    assert_eq!(required(target), Some(OperationPolicy::SpaceRead));
    assert_eq!(
        route_policy_for_target(target).unwrap().execution,
        ExecutionSelector::ScopeMode
    );
}

#[test]
fn current_client_navigation_inputs_and_permissions_are_explicit() {
    for (target, policy) in [
        ("/app/client/page/list", OperationPolicy::SpaceRead),
        ("/app/client/page/current", OperationPolicy::SpaceRead),
        ("/app/client/page/open", OperationPolicy::SpaceWrite),
    ] {
        assert_eq!(required(target), Some(policy));
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::ScopeMode
        );
    }
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/client-page.v1.schema.json")).unwrap();
    for definition in ["emptyRequest", "openRequest"] {
        schema["$ref"] = json!(format!("#/$defs/{definition}"));
        let validator = jsonschema::validator_for(&schema).unwrap();
        let mut valid = if definition == "emptyRequest" {
            json!({})
        } else {
            json!({"page":"settings"})
        };
        assert!(validator.is_valid(&valid));
        valid["clientId"] = json!("other");
        assert!(!validator.is_valid(&valid));
    }
    schema["$ref"] = json!("#/$defs/openRequest");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for input in [
        json!({"page":" "}),
        json!({"page":"settings","payload":null}),
        json!({"page":"settings","payload":[]}),
    ] {
        assert!(!validator.is_valid(&input));
    }
}
