use app_facade_api::{authorization, routing, wave};
use serde_json::{json, Value};

fn validator(name: &str) -> jsonschema::Validator {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/wave.v1.schema.json")).unwrap();
    jsonschema::validator_for(&json!({
        "$defs": schema["$defs"],
        "$ref": format!("#/$defs/{name}")
    }))
    .unwrap()
}

#[test]
fn removal_is_a_registered_hub_operation_with_manage_access() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    assert!(manifest["domains"]["wave"]["requestTargets"]
        .as_array()
        .unwrap()
        .contains(&json!(wave::REMOVE)));
    assert_eq!(
        authorization::required(wave::REMOVE),
        Some(authorization::OperationPolicy::SpaceManage)
    );
    assert!(!authorization::allows_service(wave::REMOVE));
    assert_eq!(
        routing::route_policy_for_target(wave::REMOVE)
            .unwrap()
            .execution,
        routing::ExecutionSelector::Hub
    );
    assert!(include_str!("../contract/wave-v1.md").contains(wave::REMOVE));
}

#[test]
fn removal_defaults_to_device_revocation_and_rejects_caller_supplied_authority() {
    let device = "11223344556677889900112233445566";
    let schema = validator("RemoveInput");
    let ordinary = json!({"deviceId": device});
    assert!(schema.is_valid(&ordinary));
    let request: wave::RemoveInput = serde_json::from_value(ordinary.clone()).unwrap();
    assert!(!request.forget_after_device_reset);
    let reset = json!({"deviceId": device, "forgetAfterDeviceReset": true});
    assert!(schema.is_valid(&reset));
    assert!(
        serde_json::from_value::<wave::RemoveInput>(reset)
            .unwrap()
            .forget_after_device_reset
    );
    for field in ["spaceId", "hubId", "address", "grant", "privateKey"] {
        let mut invalid = ordinary.clone();
        invalid[field] = json!("foreign");
        assert!(!schema.is_valid(&invalid));
        assert!(serde_json::from_value::<wave::RemoveInput>(invalid).is_err());
    }
}

#[test]
fn pending_and_terminal_removal_outcomes_fit_the_wire_contract() {
    let schema = validator("RemoveResult");
    for state in ["removing", "removed", "unbound", "forgotten"] {
        let value = json!({"deviceId": "11223344556677889900112233445566", "grantState": state});
        assert!(schema.is_valid(&value));
        let result: wave::RemoveResult = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(result).unwrap(), value);
    }
    assert!(!schema.is_valid(
        &json!({"deviceId": "11223344556677889900112233445566", "grantState": "committed"})
    ));
}

#[test]
fn saved_binding_can_report_pending_removal() {
    let value = json!({"deviceId": "11223344556677889900112233445566", "grantState": "removing"});
    assert!(validator("Binding").is_valid(&value));
    let binding: wave::Binding = serde_json::from_value(value).unwrap();
    assert_eq!(binding.grant_state, "removing");
}
