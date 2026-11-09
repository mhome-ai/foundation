//! Platform-attested community releases. Parse only after verifying the enclosing
//! release signature; a source manifest cannot grant this provenance to itself.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum PluginPublication {
    Community {
        #[serde(rename = "publisherId")]
        publisher_id: String,
        /// Published Integration IDs, acquired when this Plugin is enabled in a Space.
        integrations: Vec<String>,
    },
}

/// Account identity, not username or current Space, owns the runtime namespace.
pub fn plugin_publisher_namespace(publisher_id: &str) -> String {
    format!(
        "u-{}",
        &format!("{:x}", Sha256::digest(publisher_id.as_bytes()))[..32]
    )
}

/// Syntactic routing hint only. The signed publisher attestation still owns authorization.
pub fn is_community_plugin_id(id: &str) -> bool {
    let Some(rest) = id.strip_prefix("u-") else {
        return false;
    };
    let Some((namespace, slug)) = rest.split_once('-') else {
        return false;
    };
    namespace.len() == 32
        && namespace
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        && id.len() <= 80
        && slug.starts_with(|c: char| c.is_ascii_lowercase())
        && slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

impl PluginPublication {
    pub fn validate(&self, definition: &crate::PluginDefinition) -> Result<(), String> {
        let Self::Community {
            publisher_id,
            integrations,
        } = self;
        if publisher_id.is_empty()
            || publisher_id.len() > 256
            || publisher_id.trim() != publisher_id
            || definition.kind != crate::PluginKind::Extension
        {
            return Err("Invalid community Plugin publisher or kind".into());
        }
        let prefix = format!("{}-", plugin_publisher_namespace(publisher_id));
        let slug = definition
            .id
            .strip_prefix(&prefix)
            .filter(|v| !v.is_empty() && v.starts_with(|c: char| c.is_ascii_lowercase()))
            .ok_or("Community Plugin does not belong to its publisher namespace")?;
        if !slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
        {
            return Err("Invalid community Plugin name".into());
        }
        if integrations.len() > 100 {
            return Err("Too many Plugin Integration packages".into());
        }
        let mut unique = std::collections::BTreeSet::new();
        for id in integrations {
            let Some((domain, entity)) = id.split_once('.') else {
                return Err("Invalid Plugin Integration package ID".into());
            };
            if id.len() > 240
                || domain.is_empty()
                || entity.is_empty()
                || ![domain, entity].iter().all(|part| {
                    part.starts_with(|c: char| c.is_ascii_alphabetic())
                        && part
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                })
                || !unique.insert(id)
            {
                return Err("Invalid or duplicate Plugin Integration package ID".into());
            }
        }
        Ok(())
    }

    pub fn integrations(&self) -> &[String] {
        match self {
            Self::Community { integrations, .. } => integrations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn definition(publisher: &str) -> crate::PluginDefinition {
        let id = format!("{}-garden", plugin_publisher_namespace(publisher));
        crate::PluginDefinition {
            node_type: id.clone(),
            service_id: id.clone(),
            component_id: format!("node-{id}"),
            id,
            kind: crate::PluginKind::Extension,
            binding_cardinality: crate::NodeBindingCardinality::Single,
            display_name: "Garden".into(),
            description: "Example community service".into(),
            contributions: None,
        }
    }
    #[test]
    fn account_namespace_and_extension_kind_are_enforced() {
        let publication = PluginPublication::Community {
            publisher_id: "publisher-a".into(),
            integrations: vec!["garden.light".into()],
        };
        let mut plugin = definition("publisher-a");
        plugin.validate().unwrap();
        publication.validate(&plugin).unwrap();
        assert!(publication.validate(&definition("publisher-b")).is_err());
        plugin.kind = crate::PluginKind::Core;
        assert!(publication.validate(&plugin).is_err());
        plugin = definition("publisher-a");
        plugin.id = "camera".into();
        assert!(publication.validate(&plugin).is_err());
    }
    #[test]
    fn duplicate_or_path_like_integration_references_are_rejected() {
        for integrations in [
            vec!["garden.light".into(), "garden.light".into()],
            vec!["../garden".into()],
            vec!["garden.light.extra".into()],
        ] {
            assert!(PluginPublication::Community {
                publisher_id: "publisher-a".into(),
                integrations
            }
            .validate(&definition("publisher-a"))
            .is_err());
        }
    }
}
