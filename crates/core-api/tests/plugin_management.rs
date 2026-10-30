use serde_json::{json, Value};

fn validator(name: &str) -> jsonschema::Validator {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/plugin-management.v1.schema.json")).unwrap();
    schema["$ref"] = json!(format!("#/$defs/{name}"));
    jsonschema::validator_for(&schema).unwrap()
}

#[test]
fn management_event_cannot_supply_publisher_identity() {
    let v = validator("event");
    let event = json!({"version":"v1","event":"inventory/changed","data":{}});
    assert!(v.is_valid(&event));
    for (field, value) in [
        ("nodeId", json!("other")),
        ("scopeId", json!("other")),
        ("version", json!("v2")),
        ("event", json!("../platform")),
    ] {
        let mut invalid = event.clone();
        invalid[field] = value;
        assert!(!v.is_valid(&invalid));
    }
}

#[test]
fn prepared_operation_separates_public_preview_from_command() {
    let v = validator("preparedOperation");
    let prepared = json!({"preview":{"title":"Remove resource","message":"Remove the selected resource","details":{"id":"one"}},"payload":{"id":"one","expectedRevision":2}});
    assert!(v.is_valid(&prepared));
    let mut invalid = prepared;
    invalid["payload"] = json!(null);
    assert!(!v.is_valid(&invalid));
}
