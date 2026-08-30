use artifact_api::{
    ArtifactKind, ArtifactReference, PutArtifactRequest, PutArtifactResponse,
    ResolveArtifactRequest, ResolveArtifactResponse,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    schema_version: u16,
    valid: Vec<ValidCase>,
    invalid: Vec<InvalidCase>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValidCase {
    name: String,
    uri: String,
    kind: String,
    mime_type: String,
    size_bytes: u64,
    width: Option<u32>,
    height: Option<u32>,
    duration_millis: Option<u64>,
}

#[derive(Deserialize)]
struct InvalidCase {
    name: String,
    uri: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveFixture {
    schema_version: u16,
    valid: Vec<ResolveCase>,
    invalid: Vec<ResolveCase>,
}

#[derive(Deserialize)]
struct ResolveCase {
    name: String,
    value: Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PutFixture {
    schema_version: u16,
    valid: Vec<ResolveCase>,
    invalid: Vec<ResolveCase>,
}

#[test]
fn reference_fixture_matches_rust_contract_and_schema() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../fixtures/artifact-reference.conformance.json"
    ))
    .unwrap();
    assert_eq!(fixture.schema_version, 1);

    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-metadata.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for case in fixture.valid {
        let reference = ArtifactReference::parse(&case.uri)
            .unwrap_or_else(|error| panic!("{} must parse: {error}", case.name));
        let metadata = reference.metadata();
        let kind = match metadata.kind() {
            ArtifactKind::Image => "image",
            ArtifactKind::Audio => "audio",
            ArtifactKind::Video => "video",
            ArtifactKind::File => "file",
        };
        assert_eq!(kind, case.kind, "{} kind", case.name);
        assert_eq!(metadata.mime_type(), case.mime_type, "{} MIME", case.name);
        assert_eq!(metadata.size_bytes(), case.size_bytes, "{} size", case.name);
        assert_eq!(metadata.width(), case.width, "{} width", case.name);
        assert_eq!(metadata.height(), case.height, "{} height", case.name);
        assert_eq!(
            metadata.duration_millis(),
            case.duration_millis,
            "{} duration",
            case.name
        );
        assert_eq!(
            reference.uri().unwrap(),
            case.uri,
            "{} canonical URI",
            case.name
        );

        let encoded = case.uri.rsplit('/').next().unwrap();
        let metadata_json: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).unwrap()).unwrap();
        assert!(validator.is_valid(&metadata_json), "{} schema", case.name);
    }

    for case in fixture.invalid {
        assert!(
            ArtifactReference::parse(&case.uri).is_err(),
            "{} must be rejected",
            case.name
        );
    }
}

#[test]
fn resolve_fixture_matches_rust_contract_and_schema() {
    let fixture: ResolveFixture = serde_json::from_str(include_str!(
        "../fixtures/artifact-resolve.conformance.json"
    ))
    .unwrap();
    assert_eq!(fixture.schema_version, 1);

    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-resolve.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for case in fixture.valid {
        assert!(
            validator.is_valid(&case.value),
            "{} must match the resolve schema",
            case.name
        );
        if case.value.get("delivery").is_some() {
            let response: ResolveArtifactResponse = serde_json::from_value(case.value)
                .unwrap_or_else(|error| panic!("{} response must parse: {error}", case.name));
            response
                .validate()
                .unwrap_or_else(|error| panic!("{} response must validate: {error}", case.name));
        } else {
            let request: ResolveArtifactRequest = serde_json::from_value(case.value)
                .unwrap_or_else(|error| panic!("{} request must parse: {error}", case.name));
            request
                .reference()
                .unwrap_or_else(|error| panic!("{} request URI must parse: {error}", case.name));
        }
    }

    for case in fixture.invalid {
        assert!(
            !validator.is_valid(&case.value),
            "{} must be rejected by the resolve schema",
            case.name
        );
    }
}

#[test]
fn put_fixture_matches_rust_contract_and_schema() {
    let fixture: PutFixture =
        serde_json::from_str(include_str!("../fixtures/artifact-put.conformance.json")).unwrap();
    assert_eq!(fixture.schema_version, 1);

    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-put.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for case in fixture.valid {
        assert!(validator.is_valid(&case.value), "{} must match", case.name);
        if case.value.get("uri").is_some() {
            let response: PutArtifactResponse = serde_json::from_value(case.value).unwrap();
            response.validate().unwrap();
        } else {
            let request: PutArtifactRequest = serde_json::from_value(case.value).unwrap();
            request.decode().unwrap();
        }
    }
    for case in fixture.invalid {
        assert!(!validator.is_valid(&case.value), "{} must fail", case.name);
    }
}
