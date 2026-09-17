#[test]
fn execution_embeds_canonical_llm_schema_without_drift() {
    let llm: serde_json::Value = serde_json::from_str(llm_api::MESSAGE_SCHEMA).unwrap();
    let execution: serde_json::Value =
        serde_json::from_str(include_str!("../schema/execution/envelope.v1.schema.json")).unwrap();
    for (name, definition) in llm["$defs"].as_object().unwrap() {
        assert_eq!(definition, &execution["$defs"][name], "{name}");
    }
}
