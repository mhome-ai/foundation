//! Transport-neutral webhook body contract shared by desktop and embedded hosts.
use crate::{ErrorResponse, ExceptionCode};
use serde_json::Value;

pub const GENERAL_WEBHOOK_ROUTE: &str = "/webhook/general/{endpoint_id}";
pub const MAX_WEBHOOK_BODY_BYTES: usize = 1024 * 1024;

pub fn decode_body(content_type: &str, body: &[u8]) -> Result<Value, ErrorResponse> {
    let media_type = content_type
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    let is_json = media_type == "application/json"
        || (media_type.starts_with("application/") && media_type.ends_with("+json"));
    if is_json {
        serde_json::from_slice(body).map_err(|error| {
            ErrorResponse::new(
                ExceptionCode::BadRequest,
                format!("Invalid webhook JSON: {error}"),
            )
        })
    } else {
        std::str::from_utf8(body)
            .map(|text| Value::String(text.to_owned()))
            .map_err(|_| {
                ErrorResponse::new(ExceptionCode::BadRequest, "Webhook body must be UTF-8 text")
            })
    }
}
