use conversation_api::RunErrorCode;
use serde_json::Value;

fn enum_values(schema: &Value) -> Vec<String> {
    schema["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn rust_catalog_matches_the_published_schema_lists() {
    let catalog: Value =
        serde_json::from_str(include_str!("../schema/run-error-code.v1.json")).unwrap();
    let frame: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v2.schema.json")).unwrap();
    let envelope: Value =
        serde_json::from_str(include_str!("../schema/execution/envelope.v1.schema.json")).unwrap();

    let expected: Vec<&str> = RunErrorCode::ALL.iter().map(|code| code.as_str()).collect();
    assert_eq!(enum_values(&catalog), expected);
    assert_eq!(enum_values(&frame["$defs"]["runErrorCode"]), expected);
    assert_eq!(enum_values(&envelope["$defs"]["run_error_code"]), expected);
}
