use artifact_api::{
    ArtifactMetadata, ImportArtifactRequest, PrepareArtifactUploadRequest,
    PrepareArtifactUploadResponse, PutArtifactRequest, PutArtifactResponse,
    ResolveArtifactResponse,
};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

fn rejects_null<T: DeserializeOwned>(base: Value, fields: &[&str]) {
    assert!(serde_json::from_value::<T>(base.clone()).is_ok());
    for field in fields {
        let mut value = base.clone();
        value[field] = Value::Null;
        assert!(
            serde_json::from_value::<T>(value).is_err(),
            "{} must reject explicit null for {field}",
            std::any::type_name::<T>()
        );
    }
}

#[test]
fn request_fields_allow_omission_but_reject_explicit_null() {
    for lifecycle in ["std", "tmp", "man"] {
        let mut put = json!({
            "kind": "AUDIO", "mimeType": "audio/ogg", "dataBase64": "eA==",
            "lifecycle": lifecycle
        });
        if lifecycle == "man" {
            put["creationId"] = "caller-A".into();
        }
        let mut upload = put.clone();
        upload.as_object_mut().unwrap().remove("dataBase64");
        upload["sizeBytes"] = 1.into();
        upload["sha256"] = "a".repeat(64).into();
        let mut import = put.clone();
        import.as_object_mut().unwrap().remove("dataBase64");
        import.as_object_mut().unwrap().remove("mimeType");
        import["sourceUrl"] = "https://example.test/audio".into();

        assert!(serde_json::from_value::<PutArtifactRequest>(put.clone())
            .unwrap()
            .decode()
            .is_ok());
        assert!(
            serde_json::from_value::<PrepareArtifactUploadRequest>(upload.clone())
                .unwrap()
                .validate()
                .is_ok()
        );
        rejects_null::<PutArtifactRequest>(put, &["lifecycle", "creationId", "durationMillis"]);
        rejects_null::<PrepareArtifactUploadRequest>(
            upload,
            &["lifecycle", "creationId", "durationMillis"],
        );
        rejects_null::<ImportArtifactRequest>(
            import.clone(),
            &["lifecycle", "creationId", "mimeType"],
        );
        let schema: Value =
            serde_json::from_str(include_str!("../schema/artifact-import.v1.schema.json")).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&import));
        for field in ["lifecycle", "creationId", "mimeType"] {
            let mut value = import.clone();
            value[field] = Value::Null;
            assert!(!validator.is_valid(&value), "import {field}");
        }
    }
}

#[test]
fn response_and_metadata_fields_reject_explicit_null() {
    let fixture: Value =
        serde_json::from_str(include_str!("../fixtures/artifact-put.conformance.json")).unwrap();
    let response = fixture["valid"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["value"].get("uri").is_some())
        .unwrap()["value"]
        .clone();
    let optional = [
        "expiresAtUnixMs",
        "hostIdentity",
        "width",
        "height",
        "durationMillis",
    ];
    rejects_null::<PutArtifactResponse>(response.clone(), &optional);
    let mut resolve = response;
    resolve["delivery"] = json!({
        "type": "SIGNED_URL", "url": "https://example.test/file", "expiresAtUnixMs": 1
    });
    rejects_null::<ResolveArtifactResponse>(resolve, &optional);
    rejects_null::<PrepareArtifactUploadResponse>(
        json!({"uploadId":"upload", "url":"https://example.test/upload", "expiresAtUnixMs":1}),
        &["hostIdentity"],
    );
    rejects_null::<ArtifactMetadata>(json!({"k":"f", "m":"text/plain", "s":1}), &["w", "h", "d"]);
}

#[test]
fn duration_bounds_match_before_and_after_upload() {
    for (kind, code, mime) in [
        ("IMAGE", "i", "image/png"),
        ("AUDIO", "a", "audio/ogg"),
        ("VIDEO", "v", "video/mp4"),
        ("FILE", "f", "text/plain"),
    ] {
        for duration in [
            None,
            Some(0),
            Some(1),
            Some(MAX_SAFE_INTEGER),
            Some(MAX_SAFE_INTEGER + 1),
            Some(u64::MAX),
        ] {
            let mut metadata = json!({"k": code, "m": mime, "s":1});
            if matches!(kind, "IMAGE" | "VIDEO") {
                metadata["w"] = 1.into();
                metadata["h"] = 1.into();
            }
            let mut put = json!({"kind": kind, "mimeType":mime, "dataBase64":"eA=="});
            let mut upload =
                json!({"kind":kind, "mimeType":mime, "sizeBytes":1, "sha256":"a".repeat(64)});
            if let Some(value) = duration {
                metadata["d"] = value.into();
                put["durationMillis"] = value.into();
                upload["durationMillis"] = value.into();
            }
            let valid = duration.is_none_or(|value| {
                matches!(kind, "AUDIO" | "VIDEO") && (1..=MAX_SAFE_INTEGER).contains(&value)
            });
            assert_eq!(
                serde_json::from_value::<ArtifactMetadata>(metadata).is_ok(),
                valid
            );
            assert_eq!(
                serde_json::from_value::<PutArtifactRequest>(put)
                    .unwrap()
                    .decode()
                    .is_ok(),
                valid,
                "put {kind} {duration:?}"
            );
            assert_eq!(
                serde_json::from_value::<PrepareArtifactUploadRequest>(upload)
                    .unwrap()
                    .validate()
                    .is_ok(),
                valid,
                "upload {kind} {duration:?}"
            );
        }
    }
}

#[test]
fn size_bounds_match_before_and_after_upload() {
    for size in [0, 1, MAX_SAFE_INTEGER, MAX_SAFE_INTEGER + 1, u64::MAX] {
        let valid = (1..=MAX_SAFE_INTEGER).contains(&size);
        let metadata = json!({"k":"f", "m":"text/plain", "s":size});
        let upload = json!({"kind":"FILE", "mimeType":"text/plain", "sizeBytes":size, "sha256":"a".repeat(64)});
        assert_eq!(
            serde_json::from_value::<ArtifactMetadata>(metadata).is_ok(),
            valid
        );
        assert_eq!(
            serde_json::from_value::<PrepareArtifactUploadRequest>(upload)
                .unwrap()
                .validate()
                .is_ok(),
            valid,
            "size {size}"
        );
    }
}
