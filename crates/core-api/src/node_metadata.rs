//! Package-owned Node definitions. This contract contains no Plugin registry.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const NODE_METADATA_TARGET: &str = "/node/metadata";
pub const NODE_METADATA_SCHEMA_VERSION: &str = "meow.node.metadata.v1";
pub const NODE_MANIFEST_MAX_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginKind {
    Core,
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeDelivery {
    Host,
    Firmware,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeBindingCardinality {
    Single,
    Multiple,
}

impl NodeBindingCardinality {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Single => "single",
            Self::Multiple => "multiple",
        }
    }
    pub fn binding_slot<'a>(self, service_instance_id: &'a str) -> &'a str {
        match self {
            Self::Single => "single",
            Self::Multiple => service_instance_id,
        }
    }
}
impl std::fmt::Display for NodeBindingCardinality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginDefinition {
    pub id: String,
    pub node_type: String,
    pub service_id: String,
    pub component_id: String,
    pub kind: PluginKind,
    pub binding_cardinality: NodeBindingCardinality,
    pub display_name: String,
    pub description: String,
}

impl PluginDefinition {
    pub fn validate(&self) -> Result<(), String> {
        let valid_id = !self.id.is_empty()
            && self.id.len() <= 80
            && self.id.starts_with(|c: char| c.is_ascii_lowercase())
            && self
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !valid_id
            || self.node_type != self.id
            || self.service_id != self.id
            || self.component_id != format!("node-{}", self.id)
            || self.display_name.trim().is_empty()
            || self.description.trim().is_empty()
        {
            return Err("Invalid Plugin definition".into());
        }
        Ok(())
    }

    /// These fields identify one Plugin across versions, platforms and delivery forms.
    pub fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id
            && self.node_type == other.node_type
            && self.service_id == other.service_id
            && self.component_id == other.component_id
            && self.kind == other.kind
            && self.binding_cardinality == other.binding_cardinality
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeReleaseManifestBundle {
    pub schema_version: u32,
    pub kind: String,
    /// Exact publisher-signed bytes. Consumers must not reserialize before verification.
    pub manifest: String,
    pub signature: String,
}

impl NodeReleaseManifestBundle {
    pub fn sha256(&self) -> String {
        format!("{:x}", Sha256::digest(self.manifest.as_bytes()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NodeMetadata {
    pub schema_version: String,
    pub delivery: NodeDelivery,
    pub node_type: String,
    pub service_id: String,
    pub service_version: String,
    pub release_manifest: NodeReleaseManifestBundle,
}

/// Verified binding state. Persisted with the binding, usable without Host/CDN access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeBindingMetadata {
    pub definition: PluginDefinition,
    pub delivery: NodeDelivery,
    pub version: String,
    pub platform: String,
    pub manifest_sha256: String,
    pub core_protocol: Option<u32>,
}

impl NodeMetadata {
    /// The caller supplies environment-scoped publisher trust. No device may choose
    /// its own trusted key. Verification precedes all use of manifest fields.
    pub fn verify(
        &self,
        verify_signature: impl FnOnce(&[u8], &str) -> Result<(), String>,
    ) -> Result<NodeBindingMetadata, String> {
        let bundle = &self.release_manifest;
        if self.schema_version != NODE_METADATA_SCHEMA_VERSION
            || bundle.schema_version != 1
            || bundle.manifest.len() > NODE_MANIFEST_MAX_BYTES
            || bundle.signature.len() > 4096
        {
            return Err("Invalid Node metadata envelope".into());
        }
        let (bundle_kind, manifest_kind) = match self.delivery {
            NodeDelivery::Host => ("meow.plugin.manifest-bundle", "meow.plugin.manifest"),
            NodeDelivery::Firmware => (
                "meow.node.firmware-manifest-bundle",
                "meow.node.firmware-manifest",
            ),
        };
        if bundle.kind != bundle_kind {
            return Err("Node delivery and manifest kind disagree".into());
        }
        verify_signature(bundle.manifest.as_bytes(), &bundle.signature)?;
        let manifest: Value = serde_json::from_str(&bundle.manifest).map_err(|e| e.to_string())?;
        if manifest["schemaVersion"] != 1 || manifest["kind"] != manifest_kind {
            return Err("Invalid signed Node manifest".into());
        }
        let component = match self.delivery {
            NodeDelivery::Host => &manifest["component"],
            NodeDelivery::Firmware => &manifest,
        };
        let definition: PluginDefinition =
            serde_json::from_value(component["nodeMetadata"]["plugin"].clone())
                .map_err(|e| format!("Invalid signed Plugin definition: {e}"))?;
        definition.validate()?;
        let version = component["version"]
            .as_str()
            .ok_or("Node manifest version missing")?;
        let platform = manifest["platform"]
            .as_str()
            .filter(|v| !v.trim().is_empty())
            .ok_or("Node manifest platform missing")?;
        if version.trim().is_empty()
            || version != self.service_version
            || definition.node_type != self.node_type
            || definition.service_id != self.service_id
            || manifest["pluginId"] != definition.id
            || manifest["componentId"] != definition.component_id
        {
            return Err("Node manifest does not describe the running service".into());
        }
        let protocol_value = &component["compatibility"]["coreProtocol"];
        let core_protocol = if protocol_value.is_null() {
            None
        } else {
            Some(
                protocol_value
                    .as_u64()
                    .and_then(|p| u32::try_from(p).ok())
                    .filter(|p| *p > 0)
                    .ok_or("Invalid Node Core protocol revision")?,
            )
        };
        match (definition.kind, core_protocol) {
            (PluginKind::Core, Some(_)) | (PluginKind::Extension, None) => {}
            _ => return Err("Plugin kind and Core protocol disagree".into()),
        }
        Ok(NodeBindingMetadata {
            definition,
            delivery: self.delivery,
            version: version.into(),
            platform: platform.into(),
            manifest_sha256: bundle.sha256(),
            core_protocol,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn host() -> NodeMetadata {
        NodeMetadata {
            schema_version: NODE_METADATA_SCHEMA_VERSION.into(),
            delivery: NodeDelivery::Host,
            node_type: "example-extension".into(),
            service_id: "example-extension".into(),
            service_version: "1.0.0".into(),
            release_manifest: serde_json::from_str(include_str!(
                "../fixtures/node-metadata/host.bundle.json"
            ))
            .unwrap(),
        }
    }
    fn verify(bytes: &[u8], signature: &str) -> Result<(), String> {
        let key = minisign_verify::PublicKey::from_base64(
            include_str!("../fixtures/node-metadata/public-key.txt").trim(),
        )
        .unwrap();
        key.verify(
            bytes,
            &minisign_verify::Signature::decode(signature).map_err(|e| e.to_string())?,
            false,
        )
        .map_err(|e| e.to_string())
    }
    #[test]
    fn android_aar_firmware_metadata_uses_its_embedded_release_version() {
        // Generated by Plugin's Android manifest publisher with a disposable test key.
        let bundle: NodeReleaseManifestBundle = serde_json::from_str(include_str!(
            "../fixtures/node-metadata/android.bundle.json"
        ))
        .unwrap();
        let metadata = NodeMetadata {
            schema_version: NODE_METADATA_SCHEMA_VERSION.into(),
            delivery: NodeDelivery::Firmware,
            node_type: "camera".into(),
            service_id: "camera".into(),
            service_version: "1.0.5".into(),
            release_manifest: bundle,
        };
        let verify_android = |bytes: &[u8], signature: &str| {
            let key = minisign_verify::PublicKey::from_base64(
                include_str!("../fixtures/node-metadata/android-public-key.txt").trim(),
            )
            .unwrap();
            key.verify(
                bytes,
                &minisign_verify::Signature::decode(signature).map_err(|e| e.to_string())?,
                false,
            )
            .map_err(|e| e.to_string())
        };
        let binding = metadata.verify(verify_android).unwrap();
        assert_eq!(binding.delivery, NodeDelivery::Firmware);
        assert_eq!(binding.platform, "android");
        assert_eq!(binding.version, "1.0.5");
        assert_eq!(binding.core_protocol, Some(2));
        assert_eq!(
            binding.definition.binding_cardinality,
            NodeBindingCardinality::Multiple
        );
        let mut wrong_version = metadata;
        wrong_version.service_version = "1.1.0".into();
        assert!(wrong_version.verify(verify_android).is_err());
    }

    #[test]
    fn installed_release_is_verified_without_a_latest_catalog() {
        let metadata = host();
        let binding = metadata.verify(verify).unwrap();
        assert_eq!(binding.definition.kind, PluginKind::Extension);
        assert_eq!(
            binding.definition.binding_cardinality,
            NodeBindingCardinality::Single
        );
        assert_eq!(binding.manifest_sha256.len(), 64);
        assert_eq!(
            binding
                .definition
                .binding_cardinality
                .binding_slot("service-a"),
            "single"
        );
        assert_eq!(
            serde_json::from_str::<NodeBindingMetadata>(&serde_json::to_string(&binding).unwrap())
                .unwrap(),
            binding
        );
    }
    #[test]
    fn wrong_running_identity_version_delivery_and_signature_are_rejected() {
        let original = host();
        for field in ["nodeType", "serviceId", "serviceVersion", "delivery"] {
            let mut value = serde_json::to_value(&original).unwrap();
            value[field] = json!(if field == "delivery" {
                "firmware"
            } else {
                "other"
            });
            assert!(
                serde_json::from_value::<NodeMetadata>(value)
                    .unwrap()
                    .verify(verify)
                    .is_err(),
                "{field}"
            );
        }
        let mut tampered = original;
        tampered.release_manifest.manifest = tampered
            .release_manifest
            .manifest
            .replace("single", "multiple");
        assert!(tampered.verify(verify).is_err());
    }
    #[test]
    fn definition_identity_is_independent_of_version_platform_and_display_text() {
        let a = host().verify(verify).unwrap().definition;
        let mut b = a.clone();
        b.display_name = "New name".into();
        assert!(a.same_identity(&b));
        b.binding_cardinality = NodeBindingCardinality::Multiple;
        assert!(!a.same_identity(&b));
        b = a.clone();
        b.kind = PluginKind::Core;
        assert!(!a.same_identity(&b));
    }
    #[test]
    fn authentication_requires_metadata_and_acknowledges_the_exact_release() {
        let metadata = host();
        let binding = metadata.verify(verify).unwrap();
        let mut wire = json!({"metadata":metadata,"hubId":"hub","token":"token","nodeType":metadata.node_type,
            "nodeId":"node","hostId":"host","fingerprint":"fingerprint"});
        let request: crate::NodeAuthRequest = serde_json::from_value(wire.clone()).unwrap();
        wire.as_object_mut().unwrap().remove("metadata");
        assert!(serde_json::from_value::<crate::NodeAuthRequest>(wire).is_err());
        let response: crate::NodeAuthResponse =
            serde_json::from_value(json!({"ok":true,"bindingMetadata":binding})).unwrap();
        assert_eq!(response.admitted_metadata(&request).unwrap(), &binding);
        let envelope = crate::MwsMessage::create(
            None,
            "/node/auth".into(),
            "auth".into(),
            serde_json::to_string(&request).unwrap(),
            "nrq".into(),
        );
        assert!(serde_json::to_vec(&envelope).unwrap().len() < 32768);
        for field in ["version", "manifestSha256", "delivery", "coreProtocol"] {
            let mut bad = serde_json::to_value(&response).unwrap();
            bad["bindingMetadata"][field] = match field {
                "delivery" => json!("firmware"),
                "coreProtocol" => json!(2),
                _ => json!("other"),
            };
            assert!(
                serde_json::from_value::<crate::NodeAuthResponse>(bad)
                    .unwrap()
                    .admitted_metadata(&request)
                    .is_err(),
                "{field}"
            );
        }
        let missing: crate::NodeAuthResponse = serde_json::from_value(json!({"ok":true})).unwrap();
        assert!(missing.admitted_metadata(&request).is_err());
    }
}
