use app_facade_api::{authorization, developer_contracts::*, routing};
use serde_json::json;

#[test]
fn contracts_require_a_hub_and_space_read_authority() {
    for target in [LIST_TARGET, EXPORT_TARGET] {
        assert_eq!(
            routing::route_policy_for_target(target).unwrap().execution,
            routing::ExecutionSelector::Hub
        );
        assert_eq!(
            authorization::required(target),
            authorization::required("/app/integration/preview/status")
        );
    }
}

#[test]
fn input_cannot_select_space_channel_versions_or_preview() {
    let request: ExportRequest =
        serde_json::from_value(json!({"selected":["basic.device.command.onOff"]})).unwrap();
    request.validate().unwrap();
    for field in [
        "spaceId",
        "schemaMode",
        "environment",
        "version",
        "snapshot",
    ] {
        let mut value = serde_json::to_value(&request).unwrap();
        value[field] = json!("forbidden");
        assert!(serde_json::from_value::<ExportRequest>(value).is_err());
    }
    for selected in [
        vec![],
        vec!["basic.device.command.onOff", "basic.device.command.onOff"],
        vec!["basic.device.command.bad/name"],
    ] {
        let request: ExportRequest = serde_json::from_value(json!({"selected":selected})).unwrap();
        assert!(request.validate().is_err());
    }
    assert!(serde_json::from_value::<ExportRequest>(
        json!({"selected":["basic.device.command.onOff"],"ifNoneMatch":"latest"})
    )
    .unwrap()
    .validate()
    .is_err());
}
