use app_facade_api::{authorization, routing};
use serde_json::{json, Value};
fn validator(name: &str) -> jsonschema::Validator {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/matter-management.v1.schema.json")).unwrap();
    schema["$ref"] = json!(format!("#/$defs/{name}"));
    jsonschema::validator_for(&schema).unwrap()
}
#[test]
fn detail_requires_bounded_unique_identifiers() {
    let v = validator("nodeDetailRequest");
    assert!(v.is_valid(&json!({"matterNodeIds":["18446744073709551615"]})));
    for value in [
        json!({"nodeId":"1"}),
        json!({"matterNodeIds":[]}),
        json!({"matterNodeIds":["1","1"]}),
        json!({"matterNodeIds":vec!["1";21]}),
        json!({"matterNodeIds":["1"],"scopeId":"other"}),
    ] {
        assert!(!v.is_valid(&value));
    }
}
#[test]
fn partial_inventory_and_per_item_failure_have_explicit_contracts() {
    assert!(validator("inventoryResponse").is_valid(&json!({"spaceId":"s","fabric":null,"membershipComplete":false,"inventoryComplete":false,"nodes":[{"matterNodeId":"1","summary":null,"inventoryComplete":null,"devices":[]}],"errors":[{"source":"fabric","message":"offline"}]})));
    assert!(!validator("inventoryResponse").is_valid(&json!({"ok":true,"data":[]})));
    assert!(validator("nodeDetailResponse").is_valid(&json!({"spaceId":"s","items":[{"matterNodeId":"1","node":null,"error":{"message":"offline"}}]})));
}
#[test]
fn matter_reads_are_hub_owned_and_old_detail_is_not_a_target() {
    assert!(authorization::required("/app/matter/fabric/node/get").is_none());
    for target in [
        "/app/matter/status",
        "/app/matter/device/list",
        "/app/matter/fabric/node/detail",
        "/app/matter/onboarding/start",
    ] {
        assert!(authorization::required(target).is_some());
        assert_eq!(
            routing::route_policy_for_target(target).unwrap().execution,
            routing::ExecutionSelector::Hub
        );
    }
}
