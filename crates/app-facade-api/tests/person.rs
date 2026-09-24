use app_facade_api::person::*;
use serde_json::json;
#[test]
fn person_inputs_are_scope_neutral_and_strict() {
    let valid = json!({"expectedEpoch":"e","name":"Alice","tags":["Family"],"notes":"","idempotencyKey":"creation-1"});
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/person.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema["$defs"]["PersonCreate"]).unwrap();
    assert!(validator.is_valid(&valid));
    assert!(serde_json::from_value::<PersonCreate>(valid.clone()).is_ok());
    let mut invalid = valid;
    invalid["scopeId"] = json!("another-space");
    assert!(!validator.is_valid(&invalid));
    assert!(serde_json::from_value::<PersonCreate>(invalid).is_err());
    assert!(serde_json::from_value::<PersonUpdate>(
        json!({"id":"p","name":"Alice","tags":[],"notes":""})
    )
    .is_err());
}
#[test]
fn person_manifest_routes_all_management_operations_to_hub() {
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    let routing: serde_json::Value =
        serde_json::from_str(include_str!("../manifest/routing.v1.json")).unwrap();
    let targets = manifest["domains"]["person"]["requestTargets"]
        .as_array()
        .unwrap();
    assert_eq!(targets.len(), 23);
    let routing = serde_json::to_string(&routing).unwrap();
    for target in targets {
        assert!(routing.contains(target.as_str().unwrap()));
    }
    assert_eq!(
        manifest["domains"]["person"]["managementPermission"],
        "spaceMember"
    );
}
