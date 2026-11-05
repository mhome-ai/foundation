//! Complete development material. Provider values travel separately and never enter the snapshot.
use crate::extension_sources::{IntegrationSource, PackageType, SnapshotId, SourceReadResponse};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const APPLY_TARGET: &str = "/app/integration/preview/apply";
pub const STATUS_TARGET: &str = "/app/integration/preview/status";
pub const ENV_READ_ROUTE: &str = "/dispatch/integration/preview/env/read";
pub const MAX_PACKAGE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_CONFIGURATION_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewManifest {
    pub format_version: u32,
    pub integration_id: String,
    pub schemas: BTreeMap<String, String>,
    pub i18n: BTreeMap<String, String>,
    pub scripts: BTreeMap<String, script_api::ScriptArtifact>,
}

impl PreviewManifest {
    pub fn material(&self) -> Result<SourceReadResponse, &'static str> {
        if self.format_version != 1 || self.scripts.len() > 100 {
            return Err("Unsupported Preview package format");
        }
        let encoded = serde_json::to_vec(self).map_err(|_| "Invalid Preview package")?;
        if encoded.len() > MAX_PACKAGE_BYTES {
            return Err("Preview package exceeds the size limit");
        }
        let snapshot: SnapshotId = format!("{:x}", Sha256::digest(encoded)).parse()?;
        let material = SourceReadResponse {
            source: IntegrationSource {
                integration_id: self.integration_id.clone(),
                version: None,
                package_type: PackageType::Standard,
                snapshot: Some(snapshot),
            },
            schemas: self.schemas.clone(),
            i18n: self.i18n.clone(),
            script_digests: self
                .scripts
                .iter()
                .map(|(name, script)| (name.clone(), script.sha256.clone()))
                .collect(),
        };
        material.validate()?;
        for (function, script) in &self.scripts {
            let reference = script_api::ScriptRef::IntegrationPreview {
                package_id: self.integration_id.clone(),
                snapshot: snapshot.to_string(),
                function: function.clone(),
                sha256: script.sha256.clone(),
            };
            script
                .validate_for(&reference)
                .map_err(|_| "Invalid Preview script metadata")?;
        }
        Ok(material)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyPreviewRequest {
    pub material: crate::resource_transfer::TextPayload,
    pub scripts: BTreeMap<String, crate::resource_transfer::TextPayload>,
    /// Missing preserves previous values; null removes an explicitly named value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration: Option<BTreeMap<String, serde_json::Value>>,
}

impl ApplyPreviewRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.scripts.len() > 100
            || serde_json::to_vec(self)
                .map_err(|_| "Invalid Preview request")?
                .len()
                > 1024 * 1024
        {
            return Err("Preview request exceeds the inline size limit");
        }
        if let Some(configuration) = &self.configuration {
            if configuration.len() > 1000
                || serde_json::to_vec(configuration)
                    .map_err(|_| "Invalid Preview configuration")?
                    .len()
                    > MAX_CONFIGURATION_BYTES
            {
                return Err("Preview configuration exceeds the size limit");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> PreviewManifest {
        PreviewManifest {
            format_version: 1,
            integration_id: "garden.device".into(),
            schemas: BTreeMap::from([(
                "root.yml".into(),
                "integration:\n  domain: garden\n  entity: device\n".into(),
            )]),
            i18n: BTreeMap::new(),
            scripts: BTreeMap::new(),
        }
    }
    #[test]
    fn content_identity_changes_without_a_release_version_and_ignores_provider_values() {
        let mut value = manifest();
        let original = value.material().unwrap();
        assert!(original.source.version.is_none());
        assert!(original.source.snapshot.is_some());
        assert_eq!(
            original,
            serde_json::from_value::<PreviewManifest>(serde_json::to_value(&value).unwrap())
                .unwrap()
                .material()
                .unwrap()
        );
        value
            .schemas
            .get_mut("root.yml")
            .unwrap()
            .push_str("# changed\n");
        assert_ne!(
            original.source.snapshot,
            value.material().unwrap().source.snapshot
        );
        let mut raw = serde_json::to_value(&value).unwrap();
        raw["configuration"] = serde_json::json!({"token":"private"});
        assert!(serde_json::from_value::<PreviewManifest>(raw).is_err());
    }
    #[test]
    fn invalid_inventory_paths_metadata_and_inline_limits_are_rejected() {
        let mut value = manifest();
        value
            .schemas
            .insert("../outside.yml".into(), "outside".into());
        assert!(value.material().is_err());
        let mut value = manifest();
        let artifact = script_api::ScriptArtifact {
            format: script_api::ArtifactFormat::JavascriptEsm,
            sha256: "a".repeat(64),
            size: 1024,
            sdk_version: "0.2.0".into(),
            contract: None,
        };
        value.scripts.insert("../escape".into(), artifact);
        assert!(value.material().is_err());
        let request = ApplyPreviewRequest {
            material: crate::resource_transfer::TextPayload::Inline {
                text: "x".repeat(1024 * 1024),
            },
            scripts: BTreeMap::new(),
            configuration: None,
        };
        assert!(request.validate().is_err());
    }
}
