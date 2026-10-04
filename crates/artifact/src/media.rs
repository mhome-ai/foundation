//! Media values refer only to committed, scope-owned artifacts.
use crate::{ArtifactKind, ArtifactReference, ArtifactReferenceError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaReference {
    pub uri: String,
}

impl MediaReference {
    pub fn reference(
        &self,
        kind: ArtifactKind,
    ) -> Result<ArtifactReference, ArtifactReferenceError> {
        let reference = ArtifactReference::parse(&self.uri)?;
        if reference.metadata().kind() != kind {
            return Err(super::invalid(
                "media artifact kind does not match the field type",
            ));
        }
        Ok(reference)
    }

    pub fn validate(
        &self,
        tenant: &str,
        scope: &str,
        kind: ArtifactKind,
    ) -> Result<ArtifactReference, ArtifactReferenceError> {
        let reference = self.reference(kind)?;
        reference.ensure_scope(tenant, scope)?;
        Ok(reference)
    }
}

/// Explicit import of a finite HTTP resource. Scope comes from authenticated context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ImportArtifactRequest {
    pub kind: ArtifactKind,
    pub source_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transform: Option<crate::ImageTransform>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn media_rejects_legacy_fields_and_wrong_scope_or_kind() {
        for value in [
            serde_json::json!({"url":"https://example.com/a"}),
            serde_json::json!({"uri":"x","base64":"eA=="}),
            serde_json::json!({"uri":"x","mimeType":"image/png"}),
            serde_json::json!({"uri":"x","createdAt":1}),
        ] {
            assert!(serde_json::from_value::<MediaReference>(value).is_err());
        }
        let reference = ArtifactReference::new(
            "tenant",
            "scope",
            "a".repeat(64),
            crate::ArtifactMetadata::audio("audio/mpeg", 10, None).unwrap(),
        )
        .unwrap();
        let media = MediaReference {
            uri: reference.uri().unwrap(),
        };
        assert!(media
            .validate("tenant", "scope", ArtifactKind::Audio)
            .is_ok());
        assert!(media
            .validate("tenant", "other", ArtifactKind::Audio)
            .is_err());
        assert!(media
            .validate("tenant", "scope", ArtifactKind::Image)
            .is_err());
        assert!(MediaReference {
            uri: "https://example.com/a".into()
        }
        .reference(ArtifactKind::Audio)
        .is_err());
    }
}
