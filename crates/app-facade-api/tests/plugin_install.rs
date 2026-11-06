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

#[test]
fn native_preview_preflight_is_management_only_and_hub_routed() {
    assert_eq!(
        routing::route_policy_for_target(plugin::PREVIEW_CHECK_TARGET)
            .unwrap()
            .execution,
        routing::ExecutionSelector::Hub
    );
    assert_eq!(
        authorization::required(plugin::PREVIEW_CHECK_TARGET),
        Some(OperationPolicy::SpaceManage)
    );
    assert_eq!(
        routing::route_policy_for_target(plugin::PREVIEW_CHECK_TARGET),
        routing::route_policy_for_target(plugin::ENABLE_START_TARGET)
    );
    assert_eq!(
        authorization::required("/app/integration/preview/status"),
        Some(OperationPolicy::SpaceRead)
    );
    assert!(serde_json::from_value::<plugin::CheckPluginPreviewInput>(json!({})).is_ok());
    assert!(
        serde_json::from_value::<plugin::CheckPluginPreviewInput>(json!({"manage":true})).is_err()
    );
    let response: plugin::CheckPluginPreviewOutput =
        serde_json::from_value(json!({"spaceId":"home"})).unwrap();
    assert_eq!(response.space_id, "home");
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    assert!(manifest["domains"]["plugin"]["exactRequestTargets"]
        .as_array()
        .unwrap()
        .contains(&json!(plugin::PREVIEW_CHECK_TARGET)));
}
