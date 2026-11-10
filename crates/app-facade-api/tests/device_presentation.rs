use app_facade_api::{
    authorization::{required, OperationPolicy},
    routing::{route_policy_for_target, ExecutionSelector},
};

#[test]
fn device_ui_routes_have_one_owner_and_space_read_admission() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    let targets = manifest["domains"]["devicePresentation"]["uiRequestTargets"]
        .as_array()
        .unwrap();
    assert_eq!(targets.len(), 3);
    for target in targets {
        let target = target.as_str().unwrap();
        assert!(target.starts_with("/app/presentation/device/"));
        assert_eq!(required(target), Some(OperationPolicy::SpaceRead));
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::SpaceMode
        );
    }
    assert!(manifest["domains"]["resourceRead"]
        .get("uiRequestTargets")
        .is_none());
    for retired in [
        "/app/device/view/get",
        "/app/device/history/series/get",
        "/app/device/online/list",
    ] {
        assert_eq!(required(retired), None, "{retired}");
    }
    assert_eq!(
        required("/app/device/state/get"),
        Some(OperationPolicy::SpaceRead)
    );
}
