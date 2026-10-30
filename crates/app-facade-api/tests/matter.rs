use app_facade_api::{authorization, routing};
#[test]
fn matter_management_uses_generic_plugin_routing() {
    for route in [
        "status",
        "device/list",
        "fabric/node/detail",
        "onboarding/start",
    ] {
        assert!(authorization::required(&format!("/app/matter/{route}")).is_none());
    }
    for route in [
        "status",
        "inventory/get",
        "fabric/node/detail",
        "onboarding/start",
    ] {
        let target = format!("/app/plugin/matter/{route}");
        assert!(authorization::required(&target).is_some());
        assert_eq!(
            routing::route_policy_for_target(&target).unwrap().execution,
            routing::ExecutionSelector::Hub
        );
    }
}
