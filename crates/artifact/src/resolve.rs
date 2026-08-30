use std::fmt;

use serde::{Deserialize, Serialize};

use crate::{ArtifactKind, ArtifactReference};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveArtifactRequest {
    pub uri: String,
}

impl ResolveArtifactRequest {
    pub fn reference(&self) -> Result<ArtifactReference, crate::ArtifactReferenceError> {
        ArtifactReference::parse(&self.uri)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
pub enum ArtifactDelivery {
    DataUrl {
        #[serde(rename = "dataUrl")]
        data_url: String,
    },
    SignedUrl {
        url: String,
        #[serde(rename = "expiresAtUnixMs")]
        expires_at_unix_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolveArtifactResponse {
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
    pub delivery: ArtifactDelivery,
}

impl ResolveArtifactResponse {
    pub fn validate(&self) -> Result<ArtifactReference, ResolveArtifactResponseError> {
        let reference = ArtifactReference::parse(&self.uri)
            .map_err(|_| invalid("artifact resolve URI is invalid"))?;
        let metadata = reference.metadata();
        if self.kind != metadata.kind()
            || self.mime_type != metadata.mime_type()
            || self.size_bytes != metadata.size_bytes()
            || self.width != metadata.width()
            || self.height != metadata.height()
            || self.duration_millis != metadata.duration_millis()
        {
            return Err(invalid(
                "artifact resolve metadata does not match its canonical reference",
            ));
        }
        match &self.delivery {
            ArtifactDelivery::DataUrl { data_url } => {
                let prefix = format!("data:{};base64,", self.mime_type);
                if !data_url.starts_with(&prefix) || data_url.len() == prefix.len() {
                    return Err(invalid("artifact resolve data URL is invalid"));
                }
            }
            ArtifactDelivery::SignedUrl {
                url,
                expires_at_unix_ms,
            } => {
                if !(url.starts_with("https://") || url.starts_with("http://"))
                    || *expires_at_unix_ms == 0
                {
                    return Err(invalid("artifact resolve signed URL is invalid"));
                }
            }
        }
        Ok(reference)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveArtifactResponseError(&'static str);

impl fmt::Display for ResolveArtifactResponseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for ResolveArtifactResponseError {}

fn invalid(message: &'static str) -> ResolveArtifactResponseError {
    ResolveArtifactResponseError(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILE_URI: &str = "meow-artifact://v1/tenant/scope/cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc/eyJrIjoiZiIsIm0iOiJhcHBsaWNhdGlvbi9wZGYiLCJzIjozMH0";

    #[test]
    fn validates_response_against_reference() {
        let response = ResolveArtifactResponse {
            uri: FILE_URI.to_string(),
            kind: ArtifactKind::File,
            mime_type: "application/pdf".to_string(),
            size_bytes: 30,
            width: None,
            height: None,
            duration_millis: None,
            delivery: ArtifactDelivery::SignedUrl {
                url: "https://example.test/file".to_string(),
                expires_at_unix_ms: 1,
            },
        };

        assert_eq!(response.validate().unwrap().uri().unwrap(), FILE_URI);
    }

    #[test]
    fn rejects_response_metadata_drift() {
        let response = ResolveArtifactResponse {
            uri: FILE_URI.to_string(),
            kind: ArtifactKind::File,
            mime_type: "text/plain".to_string(),
            size_bytes: 30,
            width: None,
            height: None,
            duration_millis: None,
            delivery: ArtifactDelivery::DataUrl {
                data_url: "data:text/plain;base64,eA==".to_string(),
            },
        };

        assert!(response.validate().is_err());
    }
}
