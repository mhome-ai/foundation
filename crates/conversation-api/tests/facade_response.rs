use conversation_api::execution::{FacadeResponse, wire::Envelope};
use serde_json::{Value, json};

#[test]
fn response_fixtures_match_rust_types_schema_and_execution_envelopes() {
    let fixtures: Value =
        serde_json::from_str(include_str!("../fixtures/response/conformance.json")).unwrap();
    let schema: Value =
        serde_json::from_str(include_str!("../schema/execution/envelope.v1.schema.json")).unwrap();
    let response_schema = json!({
        "$schema": schema["$schema"],
        "$defs": schema["$defs"],
        "$ref": "#/$defs/facade_response",
    });
    let response_validator = jsonschema::validator_for(&response_schema).unwrap();
    let envelope_validator = jsonschema::validator_for(&schema).unwrap();
    let template: Value = serde_json::from_str(include_str!(
        "../fixtures/execution/app-facade-prepared.v1.json"
    ))
    .unwrap();

    for (section, expected) in [("valid", true), ("invalid", false)] {
        for case in fixtures[section].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let value = case["value"].clone();
            assert_eq!(
                response_validator.is_valid(&value),
                expected,
                "schema: {name}"
            );
            let decoded = serde_json::from_value::<FacadeResponse>(value.clone());
            assert_eq!(decoded.is_ok(), expected, "type: {name}");
            if let Ok(decoded) = decoded {
                let serialized = serde_json::to_value(decoded).unwrap();
                assert!(
                    response_validator.is_valid(&serialized),
                    "roundtrip: {name}"
                );
            }
            let mut envelope = template.clone();
            envelope["payload"] = json!({
                "family": "app_facade_response",
                "payload": {"type": "result", "result": {"value": value}},
            });
            assert_eq!(
                envelope_validator.is_valid(&envelope),
                expected,
                "envelope schema: {name}"
            );
            assert_eq!(
                Envelope::decode_json(&envelope.to_string()).is_ok(),
                expected,
                "envelope type: {name}"
            );
        }
    }
}
