use artifact_api::{
    ArtifactUploadReceipt, CancelArtifactUploadRequest, CancelArtifactUploadResponse,
    CompleteArtifactUploadRequest, PutArtifactResponse, StartArtifactUploadRequest,
    StartArtifactUploadResponse,
};
use serde_json::{json, Value};

fn rust_accepts(operation: &str, definition: &str, value: Value) -> bool {
    match (operation, definition) {
        ("start", "request") => serde_json::from_value::<StartArtifactUploadRequest>(value)
            .is_ok_and(|v| v.validate().is_ok()),
        ("start", "response") => serde_json::from_value::<StartArtifactUploadResponse>(value)
            .is_ok_and(|v| v.validate().is_ok()),
        ("receipt", "root") => serde_json::from_value::<ArtifactUploadReceipt>(value)
            .is_ok_and(|v| v.validate().is_ok()),
        ("complete", "request") => serde_json::from_value::<CompleteArtifactUploadRequest>(value)
            .is_ok_and(|v| v.validate().is_ok()),
        ("complete", "response") => {
            serde_json::from_value::<PutArtifactResponse>(value).is_ok_and(|v| v.validate().is_ok())
        }
        ("cancel", "request") => serde_json::from_value::<CancelArtifactUploadRequest>(value)
            .is_ok_and(|v| v.validate().is_ok()),
        ("cancel", "response") => {
            serde_json::from_value::<CancelArtifactUploadResponse>(value).is_ok()
        }
        _ => panic!("unknown fixture message {operation}/{definition}"),
    }
}

#[test]
fn stream_upload_messages_match_rust_and_published_schemas() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for operation in ["start", "receipt", "complete", "cancel"] {
        let schema: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!("schema/artifact-upload-{operation}.v1.schema.json")))
                .unwrap(),
        )
        .unwrap();
        let fixture: Value = serde_json::from_slice(
            &std::fs::read(root.join(format!(
                "fixtures/artifact-upload-{operation}.conformance.json"
            )))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(fixture["schemaVersion"], 1);
        for (section, expected) in [("valid", true), ("invalid", false)] {
            for case in fixture[section].as_array().unwrap() {
                let definition = case["definition"].as_str().unwrap();
                let mut selected = schema.clone();
                if definition != "root" {
                    selected.as_object_mut().unwrap().remove("oneOf");
                    selected["$ref"] = format!("#/$defs/{definition}").into();
                }
                let validator = jsonschema::validator_for(&selected).unwrap();
                assert_eq!(
                    validator.is_valid(&case["value"]),
                    expected,
                    "schema {operation} {}",
                    case["name"]
                );
                assert_eq!(
                    rust_accepts(operation, definition, case["value"].clone()),
                    expected,
                    "Rust {operation} {}",
                    case["name"]
                );
            }
        }
    }
}

#[test]
fn completion_checks_the_original_limit_and_media_kind() {
    let mut start: StartArtifactUploadRequest = serde_json::from_value(json!({
        "kind":"FILE", "mimeType":"application/octet-stream", "maxSizeBytes":4096
    }))
    .unwrap();
    let mut complete: CompleteArtifactUploadRequest = serde_json::from_value(json!({
        "uploadId":"generated-file", "sizeBytes":4096, "sha256":"a".repeat(64)
    }))
    .unwrap();
    assert!(start.validate_completion(&complete).is_ok());
    complete.size_bytes = 4097;
    assert!(start.validate_completion(&complete).is_err());
    complete.size_bytes = 4096;
    complete.duration_millis = Some(1000);
    assert!(start.validate_completion(&complete).is_err());
    start.kind = artifact_api::ArtifactKind::Audio;
    start.mime_type = "audio/ogg".into();
    assert!(start.validate_completion(&complete).is_ok());
}

#[test]
fn identity_and_retention_are_independent_of_streaming() {
    for identity in ["auto", "man"] {
        for retention in ["std", "tmp"] {
            let mut value = json!({"kind":"FILE", "mimeType":"application/json", "maxSizeBytes":1024,
                "identity":identity, "retention":retention});
            if identity == "man" {
                value["creationId"] = "report-1".into();
            }
            let start: StartArtifactUploadRequest = serde_json::from_value(value.clone()).unwrap();
            start.validate().unwrap();
            assert_eq!(serde_json::to_value(&start).unwrap(), value);
            assert_eq!(start.policy().is_temporary(), retention == "tmp");
        }
    }
}
