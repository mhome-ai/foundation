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

#[test]
fn summary_batch_has_no_pagination_or_id_count_cap() {
    use app_facade_api::person::{SummaryImagesInput, SummaryImagesOutput};
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/person.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema["$defs"]["SummaryImagesInput"]).unwrap();
    for count in [0, 51, 1000] {
        let value = serde_json::json!({"ids": (0..count).map(|i| format!("h:00000000-0000-0000-0000-{i:012x}")).collect::<Vec<_>>()});
        assert!(validator.is_valid(&value));
        assert_eq!(
            serde_json::from_value::<SummaryImagesInput>(value)
                .unwrap()
                .ids
                .len(),
            count
        );
    }
    assert!(!validator.is_valid(&serde_json::json!({"ids":["untyped"]})));
    assert!(!validator.is_valid(&serde_json::json!({"ids":[],"cursor":"next"})));
    let output = serde_json::json!({"items":[{"id":"h:00000000-0000-0000-0000-000000000001","uri":"meow-artifact://scoped/image"}],"missingIds":["h:00000000-0000-0000-0000-000000000002"]});
    let value: SummaryImagesOutput = serde_json::from_value(output.clone()).unwrap();
    assert_eq!(serde_json::to_value(value).unwrap(), output);
    assert!(
        jsonschema::validator_for(&schema["$defs"]["SummaryImagesOutput"])
            .unwrap()
            .is_valid(&output)
    );
}

#[test]
fn summary_references_are_hub_owned() {
    use app_facade_api::routing::{route_policy_for_target, ExecutionSelector};
    let policy =
        route_policy_for_target(app_facade_api::person::SUMMARY_IMAGES_BATCH_GET_TARGET).unwrap();
    assert_eq!(policy.execution, ExecutionSelector::Hub);
}
