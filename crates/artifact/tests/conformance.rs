use artifact_api::{
    ArtifactKind, ArtifactReference, PrepareArtifactUploadRequest, PrepareArtifactUploadResponse,
    PutArtifactRequest, PutArtifactResponse, ResolveArtifactRequest, ResolveArtifactResponse,
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

type UploadFixture = PutFixture;

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
        let request_valid = serde_json::from_value::<ResolveArtifactRequest>(case.value.clone())
            .is_ok_and(|request| request.reference().is_ok());
        let response_valid = serde_json::from_value::<ResolveArtifactResponse>(case.value)
            .is_ok_and(|response| response.validate().is_ok());
        assert!(
            !request_valid && !response_valid,
            "{} must be rejected by Rust",
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
        let request_valid = serde_json::from_value::<PutArtifactRequest>(case.value.clone())
            .is_ok_and(|request| request.decode().is_ok());
        let response_valid = serde_json::from_value::<PutArtifactResponse>(case.value)
            .is_ok_and(|response| response.validate().is_ok());
        assert!(
            !request_valid && !response_valid,
            "{} must be rejected by Rust",
            case.name
        );
    }
}

#[test]
fn upload_fixture_matches_rust_contract_and_schema() {
    let fixture: UploadFixture =
        serde_json::from_str(include_str!("../fixtures/artifact-upload.conformance.json")).unwrap();
    assert_eq!(fixture.schema_version, 1);

    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-upload.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    for case in fixture.valid {
        assert!(validator.is_valid(&case.value), "{} must match", case.name);
        if case.value.get("uploadId").is_some() {
            let response: PrepareArtifactUploadResponse =
                serde_json::from_value(case.value).unwrap();
            response.validate().unwrap();
        } else {
            let request: PrepareArtifactUploadRequest = serde_json::from_value(case.value).unwrap();
            request.validate().unwrap();
        }
    }
    for case in fixture.invalid {
        assert!(!validator.is_valid(&case.value), "{} must fail", case.name);
        let request_valid =
            serde_json::from_value::<PrepareArtifactUploadRequest>(case.value.clone())
                .is_ok_and(|request| request.validate().is_ok());
        let response_valid = serde_json::from_value::<PrepareArtifactUploadResponse>(case.value)
            .is_ok_and(|response| response.validate().is_ok());
        assert!(
            !request_valid && !response_valid,
            "{} must be rejected by Rust",
            case.name
        );
    }
}

#[test]
fn media_and_import_schemas_reject_legacy_shapes() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/media-reference.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let uri = ArtifactReference::new(
        "tenant",
        "scope",
        "a".repeat(64),
        artifact_api::ArtifactMetadata::audio("audio/mpeg", 3, None).unwrap(),
    )
    .unwrap()
    .uri()
    .unwrap();
    assert!(validator.is_valid(&serde_json::json!({"uri":uri})));
    for invalid in [
        serde_json::json!({"url":uri}),
        serde_json::json!({"uri":uri,"mimeType":"audio/mpeg"}),
        serde_json::json!({"uri":uri,"createdAt":1}),
        serde_json::json!({"uri":"https://example.test/a"}),
    ] {
        assert!(!validator.is_valid(&invalid));
    }
    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-import.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator
        .is_valid(&serde_json::json!({"kind":"AUDIO","sourceUrl":"https://example.test/audio"})));
    for invalid in [
        serde_json::json!({"kind":"AUDIO","url":"https://example.test/a"}),
        serde_json::json!({"kind":"AUDIO","sourceUrl":"data:audio/mpeg;base64,YWJj"}),
        serde_json::json!({"kind":"VIDEO","sourceUrl":"https://example.test/v"}),
    ] {
        assert!(!validator.is_valid(&invalid));
    }
}

#[test]
fn conversion_fields_are_rejected() {
    let value = serde_json::json!({"kind":"IMAGE","mimeType":"image/png","dataBase64":"eA==","transform":{"format":"JPEG"}});
    assert!(serde_json::from_value::<PutArtifactRequest>(value.clone()).is_err());
    let schema: Value =
        serde_json::from_str(include_str!("../schema/artifact-put.v1.schema.json")).unwrap();
    assert!(!jsonschema::validator_for(&schema).unwrap().is_valid(&value));
}

#[test]
fn modes_and_manual_retries_are_explicit_in_rust_and_schema() {
    use artifact_api::{ArtifactPolicy, ImportArtifactRequest};
    use serde_json::json;
    let id = "0123456789abcdef0123456789abcdef";
    for (schema_text, base, operation) in [
        (
            include_str!("../schema/artifact-put.v1.schema.json"),
            json!({"kind":"FILE","mimeType":"text/plain","dataBase64":"eA=="}),
            "put",
        ),
        (
            include_str!("../schema/artifact-upload.v1.schema.json"),
            json!({"kind":"FILE","mimeType":"text/plain","sizeBytes":1,"sha256":"a".repeat(64)}),
            "upload",
        ),
        (
            include_str!("../schema/artifact-import.v1.schema.json"),
            json!({"kind":"FILE","sourceUrl":"https://example.test/file"}),
            "import",
        ),
    ] {
        let schema: Value = serde_json::from_str(schema_text).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        for (mode, creation, valid) in [
            (None, None, true),
            (Some("std"), None, true),
            (Some("tmp"), None, true),
            (Some("man"), None, false),
            (Some("man"), Some(id), true),
            (None, Some(id), false),
            (Some("tmp"), Some(id), false),
            (Some("man"), Some("caller-A_1"), true),
            (
                Some("man"),
                Some("550e8400-e29b-41d4-a716-446655440000"),
                true,
            ),
            (Some("man"), Some("A"), true),
            (Some("man"), Some(""), false),
            (Some("man"), Some("../bad"), false),
            (Some("man"), Some("bad/id"), false),
            (Some("man"), Some("bad.id"), false),
            (Some("man"), Some("bad%2Fid"), false),
            (Some("man"), Some("bad id"), false),
            (Some("man"), Some("A\n"), false),
            (Some("man"), Some("猫"), false),
            (Some("man"), Some("a".repeat(128).as_str()), true),
            (Some("man"), Some("a".repeat(129).as_str()), false),
            (Some("unknown"), None, false),
        ] {
            let mut value = base.clone();
            if let Some(mode) = mode {
                value["identity"] = json!(if mode == "man" { "man" } else { "auto" });
                value["retention"] = json!(if mode == "man" { "std" } else { mode });
            }
            if let Some(id) = creation {
                value["creationId"] = json!(id);
            }
            assert_eq!(validator.is_valid(&value), valid, "{operation}: {value}");
            let rust_valid = match operation {
                "put" => serde_json::from_value::<PutArtifactRequest>(value.clone())
                    .map(|v| v.decode().is_ok())
                    .unwrap_or(false),
                "upload" => serde_json::from_value::<PrepareArtifactUploadRequest>(value.clone())
                    .map(|v| v.validate().is_ok())
                    .unwrap_or(false),
                _ => serde_json::from_value::<ImportArtifactRequest>(value.clone())
                    .map(|v| {
                        v.policy()
                            .validate_creation_id(v.creation_id.as_deref())
                            .is_ok()
                    })
                    .unwrap_or(false),
            };
            assert_eq!(rust_valid, valid, "{operation}: {value}");
        }
    }
    assert_eq!(ArtifactPolicy::default(), ArtifactPolicy::AUTO_STD);
}

#[test]
fn identity_and_retention_form_four_independent_resource_namespaces() {
    use artifact_api::{ArtifactIdentity, ArtifactMetadata, ArtifactPolicy, ArtifactRetention};
    let mut uris = std::collections::HashSet::new();
    for identity in [ArtifactIdentity::Auto, ArtifactIdentity::Man] {
        for retention in [ArtifactRetention::Std, ArtifactRetention::Tmp] {
            let policy = ArtifactPolicy::new(identity, retention);
            let creation = (identity == ArtifactIdentity::Man).then(|| "Caller-A".to_owned());
            let reference = ArtifactReference::with_policy(
                "tenant",
                "scope",
                "a".repeat(64),
                ArtifactMetadata::file("text/plain", 1).unwrap(),
                policy,
                creation.clone(),
            )
            .unwrap();
            let uri = reference.uri().unwrap();
            assert!(uris.insert(uri.clone()));
            let parsed = ArtifactReference::parse(&uri).unwrap();
            assert_eq!(parsed.policy(), policy);
            assert_eq!(parsed.creation_id(), creation.as_deref());
            assert_eq!(parsed.sha256(), "a".repeat(64));
            assert!(ArtifactReference::parse(&uri.replace("//v3/", "//v2/")).is_err());
        }
    }
}
