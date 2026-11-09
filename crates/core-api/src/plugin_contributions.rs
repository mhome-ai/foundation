//! Installed Plugin contribution transport. These declarations grant no operation authority.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const NODE_CONTRIBUTIONS_TARGET: &str = "/node/contributions/read";
pub const CONTRIBUTIONS_INDEX_FILE: &str = "contributions.json";
pub const CONTRIBUTIONS_INDEX_MAX_BYTES: usize = 128 * 1024;
// Base64 plus the response envelope must fit the existing 2 MiB Node message limit.
pub const CONTRIBUTION_MAX_BYTES: usize = 1024 * 1024;
pub const CONTRIBUTIONS_MAX_FILES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContributionFile {
    pub size: u64,
    pub sha256: String,
}

impl ContributionFile {
    pub fn validate(&self, maximum: usize) -> Result<(), String> {
        if self.size > maximum as u64 || !crate::plugin_preview::is_sha256(&self.sha256) {
            return Err("Invalid Plugin contribution size or digest".into());
        }
        Ok(())
    }

    pub fn verify(&self, bytes: &[u8], maximum: usize) -> Result<(), String> {
        self.validate(maximum)?;
        if bytes.len() as u64 != self.size || format!("{:x}", Sha256::digest(bytes)) != self.sha256
        {
            return Err("Plugin contribution content differs from installed source".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PluginContributionIndex {
    pub format_version: u32,
    pub plugin_id: String,
    pub version: String,
    /// Public contribution paths only; never the complete native package inventory.
    pub files: BTreeMap<String, ContributionFile>,
}

impl PluginContributionIndex {
    pub fn parse(bytes: &[u8], plugin_id: &str, version: &str) -> Result<Self, String> {
        if bytes.len() > CONTRIBUTIONS_INDEX_MAX_BYTES {
            return Err("Plugin contribution index exceeds size limit".into());
        }
        let index: Self = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        if index.format_version != 1
            || index.plugin_id != plugin_id
            || index.version != version
            || index.files.is_empty()
            || index.files.len() > CONTRIBUTIONS_MAX_FILES
        {
            return Err("Invalid Plugin contribution index identity or file count".into());
        }
        for (path, file) in &index.files {
            if !is_contribution_path(path) {
                return Err("Invalid public Plugin contribution path".into());
            }
            file.validate(CONTRIBUTION_MAX_BYTES)?;
        }
        Ok(index)
    }
}

pub fn is_contribution_path(path: &str) -> bool {
    if path.is_empty()
        || path.len() > 512
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == ".." || part.starts_with('.'))
    {
        return false;
    }
    path == "extension-package.json"
        || path.starts_with("contracts/")
        || path.starts_with("skills/")
        || path.starts_with("ui/")
        || path.starts_with("assets/")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReadNodeContribution {
    /// Exact admitted source, including same-version Preview replacements.
    pub source_manifest_sha256: String,
    /// None reads the signed index. Some reads exactly one indexed contribution.
    pub resource: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeContributionContent {
    pub source_manifest_sha256: String,
    pub resource: Option<String>,
    pub content_base64: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn source_digest_size_and_safe_public_paths_are_required() {
        let bytes = b"public contract";
        let file = ContributionFile {
            size: bytes.len() as u64,
            sha256: format!("{:x}", Sha256::digest(bytes)),
        };
        file.verify(bytes, CONTRIBUTION_MAX_BYTES).unwrap();
        assert!(file
            .verify(b"altered contract", CONTRIBUTION_MAX_BYTES)
            .is_err());
        assert!(file.verify(bytes, 1).is_err());
        let mut raw = json!({"formatVersion":1,"pluginId":"example","version":"1.0.0","files":{"skills/native.md":file}});
        assert!(PluginContributionIndex::parse(
            &serde_json::to_vec(&raw).unwrap(),
            "example",
            "1.0.0"
        )
        .is_ok());
        assert!(PluginContributionIndex::parse(
            &serde_json::to_vec(&raw).unwrap(),
            "other",
            "1.0.0"
        )
        .is_err());
        for path in [
            "../identity.json",
            "bin/example",
            "skills/../../key",
            "skills/.env",
            "ui//main.js",
            "/ui/main.js",
            "ui\\main.js",
        ] {
            let reference = raw["files"]["skills/native.md"].clone();
            raw["files"] = json!({path: reference});
            assert!(
                PluginContributionIndex::parse(
                    &serde_json::to_vec(&raw).unwrap(),
                    "example",
                    "1.0.0"
                )
                .is_err(),
                "{path}"
            );
        }
    }
}
