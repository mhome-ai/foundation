use app_facade_api::{
    authorization::{self, OperationPolicy},
    plugin, routing,
};
use serde_json::{json, Value};
#[test]
fn install_contract_requires_management_and_preserves_single_and_multiple_selection_rules() {
    for target in [
        plugin::INSTALL_PREPARE_TARGET,
        plugin::INSTALL_START_TARGET,
        plugin::INSTALL_STATUS_TARGET,
    ] {
        assert_eq!(
            authorization::required(target),
            Some(OperationPolicy::SpaceManage)
        );
        assert_eq!(
            routing::route_policy_for_target(target),
            routing::route_policy_for_target(plugin::ENABLE_START_TARGET)
        );
    }
    let schema: Value =
        serde_json::from_str(include_str!("../schema/plugin-install.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({"nodeType":"camera"})));
    assert!(validator.is_valid(&json!({"sessionId":"session","hostIds":["a","b"]})));
    assert!(!validator.is_valid(&json!({"sessionId":"session","hostIds":["a","a"]})));
    assert!(!validator.is_valid(&json!({"sessionId":"session","hostIds":[]})));
    assert!(!validator.is_valid(&json!({"sessionId":"session","retry":true})));
}
