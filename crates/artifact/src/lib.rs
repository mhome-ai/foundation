//! Stable, storage-independent artifact references shared across mHome runtimes.

use std::fmt;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Prefix of the version 1 artifact URI format.
pub const ARTIFACT_URL_PREFIX: &str = "meow-artifact://v1/";
const MAX_URI_LENGTH: usize = 2_048;
const MAX_SEGMENT_LENGTH: usize = 256;
const MAX_MIME_LENGTH: usize = 255;
const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_DIMENSION: u32 = i32::MAX as u32;

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

    pub fn from_code(value: &str) -> Result<Self, ArtifactReferenceError> {
        match value {
            "i" => Ok(Self::Image),
            "a" => Ok(Self::Audio),
            "f" => Ok(Self::File),
            _ => Err(invalid("unsupported artifact kind")),
        }
    }
}

/// Immutable metadata encoded into an artifact URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactMetadata {
    kind: ArtifactKind,
    mime_type: String,
    size_bytes: u64,
    width: Option<u32>,
    height: Option<u32>,
    duration_millis: Option<u64>,
}

impl ArtifactMetadata {
    pub fn image(
        mime_type: impl Into<String>,
        size_bytes: usize,
        width: u32,
        height: u32,
    ) -> Result<Self, ArtifactReferenceError> {
        Self::build(
            ArtifactKind::Image,
            mime_type,
            size_bytes,
            Some(width),
            Some(height),
            None,
        )
    }

    pub fn audio(
        mime_type: impl Into<String>,
        size_bytes: usize,
        duration_millis: Option<u64>,
    ) -> Result<Self, ArtifactReferenceError> {
        Self::build(
            ArtifactKind::Audio,
            mime_type,
            size_bytes,
            None,
            None,
            duration_millis,
        )
    }

    pub fn file(
        mime_type: impl Into<String>,
        size_bytes: usize,
    ) -> Result<Self, ArtifactReferenceError> {
        Self::build(ArtifactKind::File, mime_type, size_bytes, None, None, None)
    }

    fn build(
        kind: ArtifactKind,
        mime_type: impl Into<String>,
        size_bytes: usize,
        width: Option<u32>,
        height: Option<u32>,
        duration_millis: Option<u64>,
    ) -> Result<Self, ArtifactReferenceError> {
        let metadata = Self {
            kind,
            mime_type: mime_type.into(),
            size_bytes: size_bytes as u64,
            width,
            height,
            duration_millis,
        };
        metadata.validate()?;
        Ok(metadata)
    }

    #[must_use]
    pub const fn kind(&self) -> ArtifactKind {
        self.kind
    }

    #[must_use]
    pub fn mime_type(&self) -> &str {
        &self.mime_type
    }

    #[must_use]
    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    #[must_use]
    pub const fn width(&self) -> Option<u32> {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> Option<u32> {
        self.height
    }

    #[must_use]
    pub const fn duration_millis(&self) -> Option<u64> {
        self.duration_millis
    }

    fn validate(&self) -> Result<(), ArtifactReferenceError> {
        validate_mime_type(&self.mime_type)?;
        if self.size_bytes == 0 || self.size_bytes > MAX_SAFE_INTEGER {
            return Err(invalid("artifact size is invalid"));
        }
        match self.kind {
            ArtifactKind::Image => {
                if !self.mime_type.starts_with("image/") {
                    return Err(invalid("image artifact MIME type is invalid"));
                }
                if self
                    .width
                    .is_none_or(|value| value == 0 || value > MAX_DIMENSION)
                    || self
                        .height
                        .is_none_or(|value| value == 0 || value > MAX_DIMENSION)
                    || self.duration_millis.is_some()
                {
                    return Err(invalid("image artifact metadata is invalid"));
                }
            }
            ArtifactKind::Audio => {
                if !self.mime_type.starts_with("audio/") {
                    return Err(invalid("audio artifact MIME type is invalid"));
                }
                if self.width.is_some()
                    || self.height.is_some()
                    || self
                        .duration_millis
                        .is_some_and(|value| value == 0 || value > MAX_SAFE_INTEGER)
                {
                    return Err(invalid("audio artifact metadata is invalid"));
                }
            }
            ArtifactKind::File => {
                if self.mime_type.starts_with("video/") {
                    return Err(invalid("video artifacts are not supported"));
                }
                if self.width.is_some() || self.height.is_some() || self.duration_millis.is_some() {
                    return Err(invalid("file artifact metadata is invalid"));
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawArtifactMetadata {
    k: String,
    m: String,
    s: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    w: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    h: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    d: Option<u64>,
}

impl Serialize for ArtifactMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        RawArtifactMetadata {
            k: self.kind.code().to_string(),
            m: self.mime_type.clone(),
            s: self.size_bytes,
            w: self.width,
            h: self.height,
            d: self.duration_millis,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ArtifactMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawArtifactMetadata::deserialize(deserializer)?;
        let metadata = Self {
            kind: ArtifactKind::from_code(&raw.k).map_err(serde::de::Error::custom)?,
            mime_type: raw.m,
            size_bytes: raw.s,
            width: raw.w,
            height: raw.h,
            duration_millis: raw.d,
        };
        metadata.validate().map_err(serde::de::Error::custom)?;
        Ok(metadata)
    }
}

/// Canonical, scope-owned, content-addressed artifact identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReference {
    tenant_id: String,
    scope_id: String,
    sha256: String,
    metadata: ArtifactMetadata,
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
        if value.len() > MAX_URI_LENGTH {
            return Err(invalid("artifact URI is too long"));
        }
        let path = value
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
            .map_err(|_| invalid("artifact metadata is invalid"))?;
        let reference = Self::new(segments[0], segments[1], segments[2], metadata)?;
        if reference.uri()? != value {
            return Err(invalid("artifact URI is not canonical"));
        }
        Ok(reference)
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

    #[must_use]
    pub fn tenant_id(&self) -> &str {
        &self.tenant_id
    }

    #[must_use]
    pub fn scope_id(&self) -> &str {
        &self.scope_id
    }

    #[must_use]
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    #[must_use]
    pub const fn metadata(&self) -> &ArtifactMetadata {
        &self.metadata
    }

    pub fn ensure_scope(
        &self,
        tenant_id: &str,
        scope_id: &str,
    ) -> Result<(), ArtifactReferenceError> {
        if self.tenant_id != tenant_id || self.scope_id != scope_id {
            return Err(ArtifactReferenceError::new(
                ArtifactReferenceErrorKind::ScopeMismatch,
                "artifact does not belong to the current scope",
            ));
        }
        Ok(())
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
        || value.len() > MAX_SEGMENT_LENGTH
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

fn validate_mime_type(value: &str) -> Result<(), ArtifactReferenceError> {
    if value.is_empty()
        || value.len() > MAX_MIME_LENGTH
        || value != value.trim()
        || value.bytes().any(|byte| byte.is_ascii_uppercase())
    {
        return Err(invalid("artifact MIME type is invalid"));
    }
    let Some((media_type, subtype)) = value.split_once('/') else {
        return Err(invalid("artifact MIME type is invalid"));
    };
    if media_type.is_empty()
        || subtype.is_empty()
        || subtype.contains('/')
        || !value.bytes().all(|byte| {
            byte.is_ascii_lowercase()
                || byte.is_ascii_digit()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-' | b'/'
                )
        })
    {
        return Err(invalid("artifact MIME type is invalid"));
    }
    Ok(())
}

/// Stable category for reference validation failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactReferenceErrorKind {
    InvalidReference,
    ScopeMismatch,
}

/// Validation error returned for malformed or cross-scope references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactReferenceError {
    kind: ArtifactReferenceErrorKind,
    message: String,
}

impl ArtifactReferenceError {
    fn new(kind: ArtifactReferenceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    #[must_use]
    pub const fn kind(&self) -> ArtifactReferenceErrorKind {
        self.kind
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    #[must_use]
    pub const fn is_scope_mismatch(&self) -> bool {
        matches!(self.kind, ArtifactReferenceErrorKind::ScopeMismatch)
    }
}

impl fmt::Display for ArtifactReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ArtifactReferenceError {}

fn invalid(message: impl Into<String>) -> ArtifactReferenceError {
    ArtifactReferenceError::new(ArtifactReferenceErrorKind::InvalidReference, message)
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
            ArtifactMetadata::image("image/jpeg", 100, 10, 10).unwrap(),
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
    }

    #[test]
    fn rejects_cross_scope_and_invalid_metadata() {
        let reference = ArtifactReference::new(
            "tenant",
            "scope",
            "a".repeat(64),
            ArtifactMetadata::audio("audio/mpeg", 100, Some(1_000)).unwrap(),
        )
        .unwrap();

        assert_eq!(
            reference
                .ensure_scope("tenant", "other")
                .unwrap_err()
                .kind(),
            ArtifactReferenceErrorKind::ScopeMismatch
        );
        assert!(ArtifactMetadata::audio("image/png", 100, None).is_err());
        assert!(ArtifactMetadata::file("video/mp4", 100).is_err());
        assert!(ArtifactMetadata::file("Application/PDF", 100).is_err());
    }
}
