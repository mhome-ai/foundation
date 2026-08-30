use std::fmt;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};

use crate::{ArtifactKind, ArtifactReference};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutArtifactRequest {
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
}

impl PutArtifactRequest {
    pub fn decode(&self) -> Result<Vec<u8>, PutArtifactValidationError> {
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
            ArtifactKind::File if self.mime_type.starts_with("video/") => {
                return Err(invalid("video artifacts are not supported"));
            }
            _ => {}
        }
        if self.kind != ArtifactKind::Audio && self.duration_millis.is_some() {
            return Err(invalid("artifact put duration is only valid for audio"));
        }
        if self.duration_millis == Some(0) {
            return Err(invalid("artifact put duration is invalid"));
        }
        let bytes = STANDARD
            .decode(&self.data_base64)
            .map_err(|_| invalid("artifact put content is invalid base64"))?;
        if bytes.is_empty() {
            return Err(invalid("artifact put content is empty"));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PutArtifactResponse {
    pub uri: String,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_millis: Option<u64>,
}

impl PutArtifactResponse {
    pub fn validate(&self) -> Result<ArtifactReference, PutArtifactValidationError> {
        let reference = ArtifactReference::parse(&self.uri)
            .map_err(|_| invalid("artifact put URI is invalid"))?;
        let metadata = reference.metadata();
        if self.kind != metadata.kind()
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
            kind: ArtifactKind::Audio,
            mime_type: "image/png".to_string(),
            data_base64: "eA==".to_string(),
            duration_millis: None,
        };
        assert!(request.decode().is_err());
    }
}
