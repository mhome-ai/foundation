use artifact_api::{
    ArtifactDeliveryRoute, ArtifactDeliveryTransport, ArtifactUriRequest, DeleteArtifactRequest,
    ResolveArtifactRequest,
};
use serde_json::json;

#[test]
fn delivery_is_resolve_only_and_omission_preserves_the_old_request() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/artifact-resolve.conformance.json"
    ))
    .unwrap();
    let uri = fixture["valid"][0]["value"]["uri"].as_str().unwrap();
    let old = json!({"uri":uri});
    let parsed: ResolveArtifactRequest = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(parsed.delivery, None);
    assert_eq!(serde_json::to_value(parsed).unwrap(), old);
    let delete_schema: serde_json::Value = serde_json::from_str(include_str!(
        "../schema/artifact-delete-request.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&delete_schema).unwrap();
    assert!(validator.is_valid(&old));
    assert!(serde_json::from_value::<ArtifactUriRequest>(old).is_ok());
    for (route, expected) in [
        ("local", ArtifactDeliveryRoute::Local),
        ("cloud", ArtifactDeliveryRoute::Cloud),
    ] {
        let input = json!({"uri":uri,"delivery":{"transport":"https","route":route}});
        let parsed: ResolveArtifactRequest = serde_json::from_value(input.clone()).unwrap();
        let options = parsed.delivery.unwrap();
        assert_eq!(options.route, expected);
        assert_eq!(options.transport, ArtifactDeliveryTransport::Https);
        assert_eq!(serde_json::to_value(parsed).unwrap(), input);
        assert!(!validator.is_valid(&input));
        assert!(serde_json::from_value::<DeleteArtifactRequest>(input).is_err());
    }
}
