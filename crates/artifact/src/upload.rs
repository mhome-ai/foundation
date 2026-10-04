use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::ArtifactKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareArtifactUploadRequest {
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
}

impl PrepareArtifactUploadRequest {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        if self.mime_type.trim() != self.mime_type || self.mime_type.is_empty() {
            return Err(invalid("artifact upload MIME type is invalid"));
        }
        match self.kind {
            ArtifactKind::Image if !self.mime_type.starts_with("image/") => {
                return Err(invalid("artifact upload image MIME type is invalid"));
            }
            ArtifactKind::Audio if !self.mime_type.starts_with("audio/") => {
                return Err(invalid("artifact upload audio MIME type is invalid"));
            }
            ArtifactKind::Video if !self.mime_type.starts_with("video/") => {
                return Err(invalid("artifact upload video MIME type is invalid"));
            }
            ArtifactKind::File if self.mime_type.starts_with("video/") => {
                return Err(invalid("video artifacts are not supported"));
            }
            _ => {}
        }
        if self.size_bytes == 0 {
            return Err(invalid("artifact upload content is empty"));
        }
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(invalid(
                "artifact upload SHA-256 must be 64 lowercase hexadecimal characters",
            ));
        }
        if !matches!(self.kind, ArtifactKind::Audio | ArtifactKind::Video)
            && self.duration_millis.is_some()
        {
            return Err(invalid(
                "artifact upload duration is only valid for audio or video",
            ));
        }
        if self.duration_millis == Some(0) {
            return Err(invalid("artifact upload duration is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PrepareArtifactUploadResponse {
    pub upload_id: String,
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    pub expires_at_unix_ms: u64,
}

impl PrepareArtifactUploadResponse {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        if self.upload_id.trim() != self.upload_id || self.upload_id.is_empty() {
            return Err(invalid("artifact upload id is invalid"));
        }
        if !(self.url.starts_with("http://") || self.url.starts_with("https://")) {
            return Err(invalid("artifact upload URL must use HTTP or HTTPS"));
        }
        if self.expires_at_unix_ms == 0 {
            return Err(invalid("artifact upload expiration is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrepareArtifactUploadValidationError(&'static str);

impl fmt::Display for PrepareArtifactUploadValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PrepareArtifactUploadValidationError {}

fn invalid(message: &'static str) -> PrepareArtifactUploadValidationError {
    PrepareArtifactUploadValidationError(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_upload_metadata() {
        let request = PrepareArtifactUploadRequest {
            kind: ArtifactKind::Audio,
            mime_type: "audio/ogg".to_string(),
            size_bytes: 42,
            sha256: "a".repeat(64),
            duration_millis: Some(1_500),
        };
        assert!(request.validate().is_ok());
    }

    #[test]
    fn rejects_malformed_hashes_and_kind_mismatch() {
        let mut request = PrepareArtifactUploadRequest {
            kind: ArtifactKind::Audio,
            mime_type: "image/png".to_string(),
            size_bytes: 42,
            sha256: "A".repeat(64),
            duration_millis: Some(1_500),
        };
        assert!(request.validate().is_err());
        request.mime_type = "audio/ogg".to_string();
        assert!(request.validate().is_err());
    }
}
