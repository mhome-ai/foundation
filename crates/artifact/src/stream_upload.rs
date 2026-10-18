//! Finite streaming ingestion. Bytes use the HTTP data plane; these are control messages.

use serde::{Deserialize, Serialize};

use crate::upload::PrepareArtifactUploadValidationError;
use crate::{ArtifactKind, ArtifactPolicy, HostTlsIdentity, PutArtifactResponse};

object_message! {
/// Start an upload whose final length and digest need not be known yet.

pub struct StartArtifactUploadRequest {
    #[serde(default)]
    pub identity: crate::ArtifactIdentity,
    #[serde(default)]
    pub retention: crate::ArtifactRetention,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub creation_id: Option<String>,
    pub kind: ArtifactKind,
    pub mime_type: String,
    pub max_size_bytes: u64,
}
}

impl StartArtifactUploadRequest {
    pub const fn policy(&self) -> ArtifactPolicy {
        ArtifactPolicy::new(self.identity, self.retention)
    }

    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        self.policy()
            .validate_creation_id(self.creation_id.as_deref())
            .map_err(|_| invalid("invalid artifact creation ID"))?;
        crate::upload::validate_content_type(self.kind, &self.mime_type)?;
        crate::validate_size_bytes(self.max_size_bytes).map_err(invalid)
    }

    /// Validate final metadata against the immutable upload declaration.
    pub fn validate_completion(
        &self,
        completion: &CompleteArtifactUploadRequest,
    ) -> Result<(), PrepareArtifactUploadValidationError> {
        self.validate()?;
        completion.validate()?;
        if completion.size_bytes > self.max_size_bytes {
            return Err(invalid("artifact upload exceeds its declared limit"));
        }
        crate::validate_duration_millis(self.kind, completion.duration_millis).map_err(invalid)
    }
}

object_message! {
pub struct StartArtifactUploadResponse {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub host_identity: Option<HostTlsIdentity>,
    pub upload_id: String,
    pub url: String,
    pub headers: std::collections::BTreeMap<String, String>,
    pub max_size_bytes: u64,
    pub expires_at_unix_ms: u64,
}
}

impl StartArtifactUploadResponse {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        validate_upload_id(&self.upload_id)?;
        crate::validate_size_bytes(self.max_size_bytes).map_err(invalid)?;
        if self.expires_at_unix_ms == 0 || self.expires_at_unix_ms > crate::MAX_SAFE_INTEGER {
            return Err(invalid("artifact upload expiration is invalid"));
        }
        if !(self.url.starts_with("https://") || self.url.starts_with("http://")) {
            return Err(invalid("artifact upload URL must use HTTP or HTTPS"));
        }
        if self.headers.keys().any(|name| {
            name.eq_ignore_ascii_case("content-length")
                || name.eq_ignore_ascii_case("x-meow-content-sha256")
        }) {
            return Err(invalid(
                "stream upload cannot require a final size or digest header",
            ));
        }
        if let Some(host) = &self.host_identity {
            let valid_key = |key: &str| {
                key.len() == 43
                    && key
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
            };
            if host.host_id.is_empty()
                || !valid_key(&host.public_key.x)
                || !valid_key(&host.public_key.y)
            {
                return Err(invalid("artifact upload Host identity is invalid"));
            }
        }
        Ok(())
    }
}

object_message! {
/// Successful HTTP receipt means bytes were staged, not that an Artifact exists.

pub struct ArtifactUploadReceipt {
    pub upload_id: String,
    pub size_bytes: u64,
    pub sha256: String,
}
}

impl ArtifactUploadReceipt {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        validate_upload_id(&self.upload_id)?;
        crate::validate_size_bytes(self.size_bytes).map_err(invalid)?;
        crate::upload::validate_sha256(&self.sha256)
    }
}

object_message! {
/// The sender supplies the digest and length of all bytes it intended to send.

pub struct CompleteArtifactUploadRequest {
    pub upload_id: String,
    pub size_bytes: u64,
    pub sha256: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::deserialize_optional_non_null"
    )]
    pub duration_millis: Option<u64>,
}
}

impl CompleteArtifactUploadRequest {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        validate_upload_id(&self.upload_id)?;
        crate::validate_size_bytes(self.size_bytes).map_err(invalid)?;
        crate::upload::validate_sha256(&self.sha256)?;
        // The start declaration supplies the kind for the cross-request check.
        if self
            .duration_millis
            .is_some_and(|duration| duration == 0 || duration > crate::MAX_SAFE_INTEGER)
        {
            return Err(invalid("artifact duration is invalid"));
        }
        Ok(())
    }
}

pub type CompleteArtifactUploadResponse = PutArtifactResponse;

object_message! {
pub struct CancelArtifactUploadRequest {
    pub upload_id: String,
}
}

impl CancelArtifactUploadRequest {
    pub fn validate(&self) -> Result<(), PrepareArtifactUploadValidationError> {
        validate_upload_id(&self.upload_id)
    }
}

object_message! {
pub struct CancelArtifactUploadResponse {}
}

fn validate_upload_id(value: &str) -> Result<(), PrepareArtifactUploadValidationError> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_'))
    {
        return Err(invalid("artifact upload ID is invalid"));
    }
    Ok(())
}

fn invalid(message: &'static str) -> PrepareArtifactUploadValidationError {
    crate::upload::invalid(message)
}
