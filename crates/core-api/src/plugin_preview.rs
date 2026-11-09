//! Local native Preview content. Host provenance is separate from publisher trust.
use crate::{host::auth::PublicKey, PluginDefinition, PluginKind, RuntimeEnv};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const PLUGIN_PREVIEW_PROOF_TYPE: &str = "meow-plugin-preview+jwt";
pub const PLUGIN_PREVIEW_APPLY_PATH: &str = "/v1/plugins/preview/apply";
pub const PLUGIN_PREVIEW_STATUS_PATH: &str = "/v1/plugins/preview/status";
pub const PLUGIN_PREVIEW_INVENTORY_FILE: &str = "preview-files.json";
pub const PLUGIN_PREVIEW_SOURCE_FILE: &str = "preview-source.json";
pub const PLUGIN_PREVIEW_MANIFEST_FILE: &str = "preview-native.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPreviewFile {
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPreviewManifest {
    pub format_version: u32,
    pub plugin: PluginDefinition,
    pub platform: String,
    /// Diagnostic package version, never the identity of a Preview build.
    pub service_version: String,
    pub core_protocol: Option<u32>,
    /// Local platform requirements, checked by the installing Host.
    pub requires: BTreeMap<String, String>,
    pub files_sha256: String,
}

pub fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl PluginPreviewManifest {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() > crate::NODE_MANIFEST_MAX_BYTES {
            return Err("Preview manifest exceeds size limit".into());
        }
        let manifest: Self = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.plugin.validate()?;
        if self.requires.iter().any(|(component, requirement)| {
            !matches!(component.as_str(), "host" | "client") || requirement.trim().is_empty()
        }) {
            return Err("Invalid Preview platform requirements".into());
        }
        if self.format_version != 1
            || self.service_version.trim().is_empty()
            || !matches!(
                self.platform.as_str(),
                "darwin-arm64" | "darwin-x64" | "linux-arm64" | "linux-x64"
            )
            || !matches!(
                (self.plugin.kind, self.core_protocol),
                (PluginKind::Extension, None) | (PluginKind::Core, Some(1..))
            )
            || !is_sha256(&self.files_sha256)
        {
            return Err("Invalid native Preview manifest".into());
        }
        Ok(())
    }

    pub fn digest(raw: &str) -> String {
        format!("{:x}", Sha256::digest(raw.as_bytes()))
    }
}

/// The potentially large file inventory stays on the installing Host. The small
/// Node manifest carries its digest, so commissioning never transfers native files.
pub fn validate_inventory(
    plugin_id: &str,
    files: &BTreeMap<String, PluginPreviewFile>,
) -> Result<(), String> {
    if files.is_empty() || files.len() > 100_000 {
        return Err("Invalid Preview file count".into());
    }
    for (path, file) in files {
        if path.is_empty()
            || path.len() > 1024
            || path.contains('\\')
            || path.chars().any(char::is_control)
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
            || !is_sha256(&file.sha256)
        {
            return Err("Invalid native Preview file inventory".into());
        }
    }
    for required in [
        format!("bin/{}", plugin_id),
        "manifest.yaml".into(),
        "permission.yml".into(),
    ] {
        if !files.contains_key(&required) {
            return Err(format!("Preview package is missing {required}"));
        }
    }
    Ok(())
}

/// A Host assertion about installed bytes, not a Space access token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPreviewProof {
    pub host_id: String,
    pub env: RuntimeEnv,
    pub manifest_sha256: String,
}

/// Supplied only through an authenticated Space-management request. A Node's
/// metadata must never be allowed to nominate this trusted key itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPreviewExpectation {
    pub host_id: String,
    pub host_public_key: PublicKey,
    pub manifest_sha256: String,
}

/// Paths are meaningful only on the OS-authenticated local Host socket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ApplyPluginPreviewRequest {
    pub directory: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginPreviewStatusRequest {
    pub plugin_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn manifest() -> PluginPreviewManifest {
        PluginPreviewManifest {
            format_version: 1,
            plugin: PluginDefinition {
                id: "garden".into(),
                node_type: "garden".into(),
                service_id: "garden".into(),
                component_id: "node-garden".into(),
                kind: PluginKind::Extension,
                binding_cardinality: crate::NodeBindingCardinality::Multiple,
                display_name: "Garden".into(),
                description: "Garden controls".into(),
                contributions: None,
            },
            platform: "linux-arm64".into(),
            service_version: "1.0.0".into(),
            core_protocol: None,
            requires: Default::default(),
            files_sha256: "a".repeat(64),
        }
    }
    #[test]
    fn inventory_and_identity_are_checked_before_native_import() {
        let mut files: BTreeMap<String, PluginPreviewFile> =
            ["bin/garden", "manifest.yaml", "permission.yml"]
                .into_iter()
                .map(|name| {
                    (
                        name.into(),
                        PluginPreviewFile {
                            size: 1,
                            sha256: "a".repeat(64),
                        },
                    )
                })
                .collect();
        validate_inventory("garden", &files).unwrap();
        for path in ["../escape", "/absolute", "a//b", "a/./b", "a\\b", "a\u{0}"] {
            let mut candidate = files.clone();
            candidate.insert(
                path.into(),
                PluginPreviewFile {
                    size: 1,
                    sha256: "b".repeat(64),
                },
            );
            assert!(validate_inventory("garden", &candidate).is_err());
        }
        files.remove("bin/garden");
        assert!(validate_inventory("garden", &files).is_err());
        let mut candidate = manifest();
        candidate.core_protocol = Some(1);
        assert!(candidate.validate().is_err());
    }
    #[test]
    fn preview_requires_explicit_supported_platform_requirements() {
        let mut candidate = manifest();
        candidate
            .requires
            .insert("host".into(), ">=2.0.0, <3.0.0".into());
        let raw = serde_json::to_string(&candidate).unwrap();
        assert_eq!(PluginPreviewManifest::parse(&raw).unwrap(), candidate);
        let mut wire: serde_json::Value = serde_json::from_str(&raw).unwrap();
        wire.as_object_mut().unwrap().remove("requires");
        assert!(PluginPreviewManifest::parse(&wire.to_string()).is_err());
        candidate
            .requires
            .insert("node-other".into(), "=1.0.0".into());
        assert!(candidate.validate().is_err());
    }

    #[test]
    fn same_version_changed_bytes_have_different_content_identity() {
        let before = manifest();
        let mut after = before.clone();
        after.files_sha256 = "b".repeat(64);
        assert_eq!(before.service_version, after.service_version);
        assert_ne!(
            PluginPreviewManifest::digest(&serde_json::to_string(&before).unwrap()),
            PluginPreviewManifest::digest(&serde_json::to_string(&after).unwrap())
        );
    }
}
