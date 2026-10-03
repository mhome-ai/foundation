use core_api::{ErrorResponse, ExceptionCode, ExternalCoreError, HttpPayloadResponse};
use serde_json::{json, Value};

#[test]
fn http_responses_preserve_status_and_every_json_body_shape() {
    for status_code in [200, 201, 400, 401, 403, 404, 409, 413, 429, 500, 503] {
        for body in [
            Value::Null,
            json!("text"),
            json!([1, 2]),
            json!({"error":"busy"}),
        ] {
            let response = HttpPayloadResponse { status_code, body };
            let wire = serde_json::to_value(&response).unwrap();
            assert_eq!(wire["statusCode"], status_code);
            assert_eq!(
                serde_json::from_value::<HttpPayloadResponse>(wire).unwrap(),
                response
            );
        }
    }
}

#[test]
fn domain_errors_preserve_details_across_external_wire() {
    let original = ErrorResponse::new(ExceptionCode::BadRequest, "invalid action")
        .with_details(json!({"issues":[{"field":"volume"}]}));
    let external = ExternalCoreError::from(original.clone());
    let wire = serde_json::to_vec(&external).unwrap();
    let decoded: ExternalCoreError = serde_json::from_slice(&wire).unwrap();
    assert_eq!(ErrorResponse::from(decoded), original);
}

#[test]
fn external_protocol_version_matches_manifest() {
    let manifest: Value = serde_json::from_str(include_str!("../manifest/core.v1.json")).unwrap();
    fn find_version(value: &Value) -> Option<u64> {
        match value {
            Value::Object(fields) => fields
                .get("externalProtocolVersion")
                .and_then(Value::as_u64)
                .or_else(|| fields.values().find_map(find_version)),
            Value::Array(values) => values.iter().find_map(find_version),
            _ => None,
        }
    }
    assert_eq!(
        find_version(&manifest),
        Some(core_api::EXTERNAL_CORE_PROTOCOL_VERSION as u64)
    );
}

#[test]
fn webhook_json_is_json_and_other_bodies_are_exact_utf8_text() {
    use core_api::webhook::decode_body;
    for content_type in [
        "application/json",
        "Application/Problem+Json; charset=utf-8",
    ] {
        assert_eq!(decode_body(content_type, b"null").unwrap(), Value::Null);
        assert_eq!(
            decode_body(content_type, br#"{"volume":12}"#).unwrap(),
            json!({"volume":12})
        );
        assert!(decode_body(content_type, b"{bad").is_err());
    }
    for content_type in ["", "text/xml", "text/plain"] {
        let xml = "<event>音量 &amp; music</event>\n";
        assert_eq!(
            decode_body(content_type, xml.as_bytes()).unwrap(),
            json!(xml)
        );
        assert_eq!(decode_body(content_type, b"").unwrap(), json!(""));
        assert_eq!(
            decode_body(content_type, br#"{"volume":12}"#).unwrap(),
            json!("{\"volume\":12}")
        );
        assert!(decode_body(content_type, &[0xff]).is_err());
    }
}
