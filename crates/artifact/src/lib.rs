//! Stable, storage-independent artifact references shared across mHome runtimes.

use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};

/// Prefix of the version 1 artifact URI format.
pub const ARTIFACT_URL_PREFIX: &str = "meow-artifact://v1/";

/// Logical media kind encoded in an artifact reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Image,
    Audio,
    File,
}

impl ArtifactKind {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Image => "i",
            Self::Audio => "a",
            Self::File => "f",
        }
    }

    fn from_code(value: &str) -> Result<Self, ArtifactReferenceError> {
        match value {
            "i" => Ok(Self::Image),
            "a" => Ok(Self::Audio),
            "f" => Ok(Self::File),
            _ => Err(invalid("unsupported artifact kind")),
        }
    }
}

/// Immutable metadata encoded into an artifact URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub k: String,
    pub m: String,
    pub s: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub h: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub d: Option<u64>,
}

impl ArtifactMetadata {
    pub fn image(mime_type: impl Into<String>, size_bytes: usize, width: u32, height: u32) -> Self {
        Self {
            k: ArtifactKind::Image.code().to_string(),
            m: mime_type.into(),
            s: size_bytes as u64,
            w: Some(width),
            h: Some(height),
            d: None,
        }
    }

    pub fn audio(
        mime_type: impl Into<String>,
        size_bytes: usize,
        duration_millis: Option<u64>,
    ) -> Self {
        Self {
            k: ArtifactKind::Audio.code().to_string(),
            m: mime_type.into(),
            s: size_bytes as u64,
            w: None,
            h: None,
            d: duration_millis,
        }
    }

    pub fn kind(&self) -> Result<ArtifactKind, ArtifactReferenceError> {
        ArtifactKind::from_code(&self.k)
    }

    fn validate(&self) -> Result<(), ArtifactReferenceError> {
        if self.m.trim().is_empty() || self.s == 0 {
            return Err(invalid("artifact metadata is incomplete"));
        }
        match self.kind()? {
            ArtifactKind::Image if !self.m.starts_with("image/") => {
                Err(invalid("image artifact MIME type is invalid"))
            }
            ArtifactKind::Image
                if self.w.is_none_or(|value| value == 0)
                    || self.h.is_none_or(|value| value == 0) =>
            {
                Err(invalid("image artifact dimensions are missing"))
            }
            ArtifactKind::Audio if !self.m.starts_with("audio/") => {
                Err(invalid("audio artifact MIME type is invalid"))
            }
            ArtifactKind::Audio | ArtifactKind::File if self.w.is_some() || self.h.is_some() => {
                Err(invalid("non-image artifact cannot have image dimensions"))
            }
            _ => Ok(()),
        }
    }
}

/// Authenticated, content-addressed artifact identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReference {
    pub tenant_id: String,
    pub scope_id: String,
    pub sha256: String,
    pub metadata: ArtifactMetadata,
}

impl ArtifactReference {
    pub fn new(
        tenant_id: impl Into<String>,
        scope_id: impl Into<String>,
        sha256: impl Into<String>,
        metadata: ArtifactMetadata,
    ) -> Result<Self, ArtifactReferenceError> {
        let reference = Self {
            tenant_id: tenant_id.into(),
            scope_id: scope_id.into(),
            sha256: sha256.into(),
            metadata,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn parse(value: &str) -> Result<Self, ArtifactReferenceError> {
        let path = value
            .trim()
            .strip_prefix(ARTIFACT_URL_PREFIX)
            .ok_or_else(|| invalid("unsupported artifact URI"))?;
        let segments = path.split('/').collect::<Vec<_>>();
        if segments.len() != 4 {
            return Err(invalid(
                "artifact URI must contain tenant, scope, digest, and metadata",
            ));
        }
        let metadata_bytes = URL_SAFE_NO_PAD
            .decode(segments[3])
            .map_err(|_| invalid("artifact metadata is not valid base64url"))?;
        let metadata: ArtifactMetadata = serde_json::from_slice(&metadata_bytes)
            .map_err(|_| invalid("artifact metadata is not valid JSON"))?;
        Self::new(segments[0], segments[1], segments[2], metadata)
    }

    pub fn uri(&self) -> Result<String, ArtifactReferenceError> {
        self.validate()?;
        let metadata = serde_json::to_vec(&self.metadata)
            .map_err(|_| invalid("artifact metadata cannot be encoded"))?;
        Ok(format!(
            "{ARTIFACT_URL_PREFIX}{}/{}/{}/{}",
            self.tenant_id,
            self.scope_id,
            self.sha256,
            URL_SAFE_NO_PAD.encode(metadata)
        ))
    }

    pub fn ensure_scope(
        &self,
        tenant_id: &str,
        scope_id: &str,
    ) -> Result<(), ArtifactReferenceError> {
        if self.tenant_id != tenant_id || self.scope_id != scope_id {
            return Err(scope_mismatch(
                "artifact does not belong to the current scope",
            ));
        }
        Ok(())
    }

    pub fn relative_object_path(&self) -> Result<String, ArtifactReferenceError> {
        self.validate()?;
        Ok(format!(
            "v1/{}/{}/sha256/{}/{}",
            self.tenant_id,
            self.scope_id,
            &self.sha256[..2],
            self.sha256
        ))
    }

    fn validate(&self) -> Result<(), ArtifactReferenceError> {
        validate_segment(&self.tenant_id, "tenant")?;
        validate_segment(&self.scope_id, "scope")?;
        if self.sha256.len() != 64
            || !self
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        {
            return Err(invalid("artifact sha256 is invalid"));
        }
        self.metadata.validate()
    }
}

fn validate_segment(value: &str, name: &str) -> Result<(), ArtifactReferenceError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'))
    {
        return Err(invalid(format!("artifact {name} is not URL-safe")));
    }
    Ok(())
}

/// Validation error returned for malformed or cross-scope references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReferenceError {
    message: String,
    scope_mismatch: bool,
}

impl ArtifactReferenceError {
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn is_scope_mismatch(&self) -> bool {
        self.scope_mismatch
    }
}

impl fmt::Display for ArtifactReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ArtifactReferenceError {}

fn invalid(message: impl Into<String>) -> ArtifactReferenceError {
    ArtifactReferenceError {
        message: message.into(),
        scope_mismatch: false,
    }
}

fn scope_mismatch(message: impl Into<String>) -> ArtifactReferenceError {
    ArtifactReferenceError {
        message: message.into(),
        scope_mismatch: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_owned_reference_round_trips() {
        let reference = ArtifactReference::new(
            "tenant",
            "scope",
            "a".repeat(64),
            ArtifactMetadata::image("image/jpeg", 100, 10, 10),
        )
        .unwrap();
        assert_eq!(
            reference.uri().unwrap(),
            format!(
                "meow-artifact://v1/tenant/scope/{}/eyJrIjoiaSIsIm0iOiJpbWFnZS9qcGVnIiwicyI6MTAwLCJ3IjoxMCwiaCI6MTB9",
                "a".repeat(64)
            )
        );
        assert_eq!(
            ArtifactReference::parse(&reference.uri().unwrap()).unwrap(),
            reference
        );
        assert_eq!(
            reference.relative_object_path().unwrap(),
            format!("v1/tenant/scope/sha256/aa/{}", "a".repeat(64))
        );
    }

    #[test]
    fn rejects_cross_scope_and_path_like_segments() {
        let reference = ArtifactReference::new(
            "tenant",
            "scope",
            "a".repeat(64),
            ArtifactMetadata::audio("audio/mpeg", 100, Some(1_000)),
        )
        .unwrap();

        let error = reference.ensure_scope("tenant", "other").unwrap_err();
        assert!(error.is_scope_mismatch());
        assert!(ArtifactReference::new(
            "..",
            "scope",
            "a".repeat(64),
            ArtifactMetadata::audio("audio/mpeg", 100, None),
        )
        .is_err());
    }
}
