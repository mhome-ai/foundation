use std::fmt;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::{ArtifactKind, ArtifactReference};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutArtifactRequest {
    #[serde(default)]
    pub lifecycle: crate::ArtifactLifecycle,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub creation_id: Option<String>,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub duration_millis: Option<u64>,
}

impl PutArtifactRequest {
    pub fn decode(&self) -> Result<Vec<u8>, PutArtifactValidationError> {
        self.lifecycle
            .validate_creation_id(self.creation_id.as_deref())
            .map_err(|_| invalid("invalid artifact creation ID"))?;
        if self.mime_type.trim() != self.mime_type || self.mime_type.is_empty() {
            return Err(invalid("artifact put MIME type is invalid"));
        }
        match self.kind {
            ArtifactKind::Image if !self.mime_type.starts_with("image/") => {
                return Err(invalid("artifact put image MIME type is invalid"));
            }
            ArtifactKind::Audio if !self.mime_type.starts_with("audio/") => {
                return Err(invalid("artifact put audio MIME type is invalid"));
            }
            ArtifactKind::Video if !self.mime_type.starts_with("video/") => {
                return Err(invalid("artifact put video MIME type is invalid"));
            }
            ArtifactKind::File if self.mime_type.starts_with("video/") => {
                return Err(invalid("video artifacts are not supported"));
            }
            _ => {}
        }
        crate::validate_duration_millis(self.kind, self.duration_millis).map_err(invalid)?;
        let bytes = STANDARD
            .decode(&self.data_base64)
            .map_err(|_| invalid("artifact put content is invalid base64"))?;
        crate::validate_size_bytes(bytes.len() as u64).map_err(invalid)?;
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutArtifactResponse {
    #[serde(default)]
    pub lifecycle: crate::ArtifactLifecycle,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub expires_at_unix_ms: Option<u64>,
    pub uri: String,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub width: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub height: Option<u32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub duration_millis: Option<u64>,
}

impl PutArtifactResponse {
    pub fn validate(&self) -> Result<ArtifactReference, PutArtifactValidationError> {
        if self.lifecycle.is_temporary() != self.expires_at_unix_ms.is_some()
            || self.expires_at_unix_ms == Some(0)
        {
            return Err(invalid("artifact lifetime and expiration are inconsistent"));
        }
        let reference = ArtifactReference::parse(&self.uri)
            .map_err(|_| invalid("artifact put URI is invalid"))?;
        let metadata = reference.metadata();
        if self.lifecycle != reference.lifecycle()
            || self.kind != metadata.kind()
            || self.mime_type != metadata.mime_type()
            || self.size_bytes != metadata.size_bytes()
            || self.width != metadata.width()
            || self.height != metadata.height()
            || self.duration_millis != metadata.duration_millis()
        {
            return Err(invalid(
                "artifact put metadata does not match its canonical reference",
            ));
        }
        Ok(reference)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutArtifactValidationError(&'static str);

impl fmt::Display for PutArtifactValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PutArtifactValidationError {}

fn invalid(message: &'static str) -> PutArtifactValidationError {
    PutArtifactValidationError(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_rejects_kind_mime_mismatch() {
        let request = PutArtifactRequest {
            lifecycle: crate::ArtifactLifecycle::Std,
            creation_id: None,
            kind: ArtifactKind::Audio,
            mime_type: "image/png".to_string(),
            data_base64: "eA==".to_string(),
            duration_millis: None,
        };
        assert!(request.decode().is_err());
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[test]
    fn default_ingress_is_durable_and_temporary_is_explicit() {
        let mut input: serde_json::Value =
            serde_json::json!({"kind":"FILE","mimeType":"text/plain","dataBase64":"eA=="});
        let durable: PutArtifactRequest = serde_json::from_value(input.clone()).unwrap();
        assert!(!durable.lifecycle.is_temporary());
        assert_eq!(serde_json::to_value(&durable).unwrap()["lifecycle"], "std");
        input["lifecycle"] = "tmp".into();
        let temporary: PutArtifactRequest = serde_json::from_value(input).unwrap();
        assert!(temporary.lifecycle.is_temporary());
        assert_eq!(temporary.decode().unwrap(), b"x");
    }
}
