use serde_json::{json, Value};
use storage_api::*;
fn validate(kind: &str, value: &Value) {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/storage.v1.schema.json")).unwrap();
    schema["$ref"] = json!(format!("#/$defs/{kind}"));
    jsonschema::validator_for(&schema)
        .unwrap()
        .validate(value)
        .unwrap();
}
#[test]
fn immutable_path_requests_have_no_settings_or_implicit_recursive_deletion() {
    let value = json!({"folderId":"home/photos"});
    validate("createFolder", &value);
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<ObjectStorageCreateFolderRequest>(value.clone()).unwrap()
        )
        .unwrap(),
        value
    );
    assert!(
        !serde_json::from_value::<ObjectStorageDeleteFolderRequest>(value)
            .unwrap()
            .recursive
    );
    validate(
        "deleteFolder",
        &json!({"folderId":"home/photos","recursive":true}),
    );
    for field in ["name", "maxBytes", "overflowStrategy", "expectedRevision"] {
        let mut value = json!({"folderId":"home/photos"});
        value[field] = json!("legacy");
        assert!(serde_json::from_value::<ObjectStorageCreateFolderRequest>(value).is_err());
    }
    validate(
        "sessionRequest",
        &serde_json::to_value(ObjectStorageSessionRequest {
            folder_id: "home/photos".into(),
            permissions: vec![StoragePermission::Read],
            ttl_seconds: Some(900),
        })
        .unwrap(),
    );
    assert!(serde_json::from_value::<ObjectStorageSessionRequest>(
        json!({"folderId":"home/photos","permissions":["admin"]})
    )
    .is_err());
}
#[test]
fn typed_errors_and_physical_contracts_round_trip() {
    for error in [
        StorageError::new(StorageErrorCode::VolumeFull, "full"),
        StorageError::new(StorageErrorCode::Unavailable, "timeout").uncertain(),
        StorageError::new(StorageErrorCode::Unavailable, "delete failed").partial(2),
    ] {
        let value = serde_json::to_value(&error).unwrap();
        validate("error", &value);
        assert_eq!(
            serde_json::from_value::<StorageError>(value).unwrap(),
            error
        );
    }
    validate(
        "blob",
        &serde_json::to_value(backend::Blob {
            id: "id".into(),
            size_bytes: 0,
            sha256: "a".repeat(64),
        })
        .unwrap(),
    );
    assert_eq!(
        StorageError::new(StorageErrorCode::VolumeFull, "full").http_status(),
        507
    );
}

#[test]
fn management_contract_covers_system_folders_and_multiple_repositories() {
    let folder = json!({"system":true,"deleting":false,"id":"artifact","usedBytes":"5","reservedBytes":"0","objectCount":"1","createdAtMs":"1","updatedAtMs":"1"});
    let typed: ObjectStorageFolder = serde_json::from_value(folder.clone()).unwrap();
    validate("folder", &serde_json::to_value(typed).unwrap());
    validate("folderList", &json!({"folders":[folder]}));
    let capacity = json!({"totalBytes":"1000","availableBytes":"500"});
    let overview = json!({"backends":[{"kind":"internal","repositoryId":"internal-uuid","online":true,"capacity":capacity},{"kind":"external","repositoryId":"external-uuid","online":false,"capacity":null}],"protocolVersion":PROTOCOL_VERSION,"writeState":"writable","writeAvailable":true,"filesystem":capacity,"storage":{"usedBytes":"5","reservedBytes":"0","writableBytes":"400"},"safety":{"warningAtAvailableBytes":"200","stopAtAvailableBytes":"100"},"folderCount":"1","objectCount":"1","currentSpaceFolderCount":"1","currentSpaceObjectCount":"1","currentSpaceUsedBytes":"5","currentSpaceReservedBytes":"0","otherSpacesUsedBytes":"0","activeUploadCount":"0","pendingDeletionCount":"0"});
    validate("overview", &overview);
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<ObjectStorageOverview>(overview.clone()).unwrap()
        )
        .unwrap(),
        overview
    );
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/storage.v1.json")).unwrap();
    assert_eq!(manifest["contractVersion"], env!("CARGO_PKG_VERSION"));
    assert_eq!(manifest["protocol"], PROTOCOL_VERSION);
}

#[test]
fn schema_enforces_the_same_decimal_u64_boundary_as_rust() {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/storage.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/$defs/decimalU64");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for value in [
        "0",
        "1",
        "9999999999999999999",
        "10000000000000000000",
        "18446744073709551614",
        "18446744073709551615",
    ] {
        assert!(validator.is_valid(&json!(value)), "{value}");
    }
    for value in [
        "01",
        "-1",
        "18446744073709551616",
        "18446744073709551619",
        "18446744073709551620",
        "19000000000000000000",
        "99999999999999999999",
    ] {
        assert!(!validator.is_valid(&json!(value)), "{value}");
    }
}

#[test]
fn schema_enforces_namespaces_and_path_segments() {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/storage.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/$defs/folderPath");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for path in [
        "artifact",
        "home/照片",
        "home/a_b%/c",
        "plugin/camera/one/recordings",
        "core/cache/files",
    ] {
        assert!(validator.is_valid(&json!(path)), "{path}");
    }
    for path in [
        "home",
        "home/",
        "home/../a",
        "home/a//b",
        "home/ a",
        "home/a ",
        "artifact/a",
        "plugin/camera/one",
        "unknown/a",
    ] {
        assert!(!validator.is_valid(&json!(path)), "{path}");
    }
    assert!(!validator.is_valid(&json!(format!("home/{}", "x".repeat(81)))));
    assert!(!validator.is_valid(&json!(format!("home/{}", vec!["x"; 16].join("/")))));
}
