use app_facade_api::person::ClusterName;
#[test]
fn naming_requires_the_exact_card_epoch_revision_and_photo() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/person.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema["$defs"]["ClusterName"]).unwrap();
    let valid = serde_json::json!({"id":"group","name":"Alice","expectedEpoch":"epoch","expectedRevision":"4","representativeSampleId":"photo"});
    assert!(validator.is_valid(&valid));
    assert!(serde_json::from_value::<ClusterName>(valid.clone()).is_ok());
    let mut invalid = valid;
    invalid
        .as_object_mut()
        .unwrap()
        .remove("representativeSampleId");
    assert!(!validator.is_valid(&invalid));
    assert!(serde_json::from_value::<ClusterName>(invalid).is_err());
}
