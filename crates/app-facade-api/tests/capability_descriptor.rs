use app_facade_api::integration::CapabilityDescriptor;
use serde_json::Value;

#[test]
fn shared_capability_corpus_round_trips() {
    let schema: Value = serde_json::from_str(include_str!(
        "../schema/capability-descriptor.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let corpus: Value = serde_json::from_str(include_str!(
        "../fixtures/capability-descriptor.conformance.json"
    ))
    .unwrap();
    for case in corpus["valid"].as_array().unwrap() {
        let value = &case["value"];
        assert!(validator.is_valid(value), "{}", case["name"]);
        let descriptor: CapabilityDescriptor = serde_json::from_value(value.clone()).unwrap();
        descriptor.validate().unwrap();
        assert_eq!(serde_json::to_value(descriptor).unwrap(), *value);
    }
    for case in corpus["invalid"].as_array().unwrap() {
        assert!(!validator.is_valid(&case["value"]), "{}", case["name"]);
    }
}

#[test]
fn parent_is_an_actual_override_not_self_or_an_unrelated_method() {
    let mut descriptor = CapabilityDescriptor {
        capability_id: "provider.device.command.onOff".into(),
        kind: "COMMAND".into(),
        visibility: "full".into(),
        ..Default::default()
    };
    descriptor.parent_capability_id = Some(descriptor.capability_id.clone());
    assert!(descriptor.validate().is_err());
    descriptor.parent_capability_id = Some("basic.device.state.onOff".into());
    assert!(descriptor.validate().is_err());
    descriptor.parent_capability_id = Some("basic.device.command.onOff".into());
    assert!(descriptor.validate().is_ok());
}
