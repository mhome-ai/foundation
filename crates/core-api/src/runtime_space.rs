//! Device runtime admission, independent of user permissions and Node topology.
//!
//! The Host supplies this policy through its trusted local runtime configuration.
//! Discovery only advertises it; accepting a credential must always check the
//! local policy. An unclaimed single-Space Host has no effective policy yet.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpaceIdentity {
    pub tenant_id: String,
    pub space_id: String,
}

impl SpaceIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if valid_id(&self.tenant_id) && valid_id(&self.space_id) {
            Ok(())
        } else {
            Err("invalid runtime Space identity")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub enum RuntimeSpacePolicy {
    Multi {},
    Single { space: SpaceIdentity },
}

impl RuntimeSpacePolicy {
    /// Discovery is advisory, but missing or unknown restrictions are never
    /// interpreted as an unrestricted device. Authentication rechecks the policy.
    pub fn from_mdns(
        mode: Option<&str>,
        tenant: Option<&str>,
        scope: Option<&str>,
    ) -> Result<Self, &'static str> {
        let policy = match (mode, tenant, scope) {
            (Some("multi"), None, None) => Self::Multi {},
            (Some("single"), Some(tenant), Some(scope)) => Self::Single {
                space: SpaceIdentity {
                    tenant_id: tenant.into(),
                    space_id: scope.into(),
                },
            },
            _ => return Err("missing or invalid runtime Space discovery policy"),
        };
        policy.validate()?;
        Ok(policy)
    }

    pub fn mdns_properties(&self) -> Vec<(String, String)> {
        match self {
            Self::Multi {} => vec![("space_mode".into(), "multi".into())],
            Self::Single { space } => vec![
                ("space_mode".into(), "single".into()),
                ("space_tenant".into(), space.tenant_id.clone()),
                ("space_id".into(), space.space_id.clone()),
            ],
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Multi {} => Ok(()),
            Self::Single { space } => space.validate(),
        }
    }

    /// Call for initial admission, persisted-identity recovery and credential
    /// refresh. A valid credential for another Space does not override this rule.
    pub fn admit(&self, tenant_id: &str, space_id: &str) -> Result<(), &'static str> {
        self.validate()?;
        if !valid_id(tenant_id) || !valid_id(space_id) {
            return Err("invalid requested Space identity");
        }
        match self {
            Self::Multi {} => Ok(()),
            Self::Single { space }
                if space.tenant_id == tenant_id && space.space_id == space_id =>
            {
                Ok(())
            }
            Self::Single { .. } => Err("runtime is restricted to another Space"),
        }
    }
}

impl std::str::FromStr for RuntimeSpacePolicy {
    type Err = String;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let policy: Self = serde_json::from_str(value).map_err(|e| e.to_string())?;
        policy.validate().map_err(str::to_owned)?;
        Ok(policy)
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.trim() == value && value.len() <= 240
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_space_rejects_other_tenants_and_spaces() {
        let policy = RuntimeSpacePolicy::Single {
            space: SpaceIdentity {
                tenant_id: "tenant-a".into(),
                space_id: "space-a".into(),
            },
        };
        assert!(policy.admit("tenant-a", "space-a").is_ok());
        assert!(policy.admit("tenant-a", "space-b").is_err());
        assert!(policy.admit("tenant-b", "space-a").is_err());
        assert!(policy.admit("", "space-a").is_err());
        assert!(RuntimeSpacePolicy::Multi {}
            .admit("tenant-b", "space-b")
            .is_ok());
    }

    #[test]
    fn discovery_roundtrip_and_missing_policy_fail_closed() {
        let multi = RuntimeSpacePolicy::Multi {};
        let single = RuntimeSpacePolicy::Single {
            space: SpaceIdentity {
                tenant_id: "t".into(),
                space_id: "s".into(),
            },
        };
        for policy in [multi, single] {
            let properties: std::collections::BTreeMap<_, _> =
                policy.mdns_properties().into_iter().collect();
            let get = |key| properties.get(key).map(String::as_str);
            assert_eq!(
                RuntimeSpacePolicy::from_mdns(
                    get("space_mode"),
                    get("space_tenant"),
                    get("space_id")
                )
                .unwrap(),
                policy
            );
        }
        assert!(RuntimeSpacePolicy::from_mdns(None, None, None).is_err());
        assert!(RuntimeSpacePolicy::from_mdns(Some("multi"), Some("t"), Some("s")).is_err());
        assert!(RuntimeSpacePolicy::from_mdns(Some("single"), None, None).is_err());
    }

    #[test]
    fn missing_or_malformed_policy_never_defaults_to_multi() {
        for input in [
            "{}",
            "null",
            r#"{"mode":"single"}"#,
            r#"{"mode":"multi","space":{"tenantId":"t","spaceId":"s"}}"#,
        ] {
            assert!(
                serde_json::from_str::<RuntimeSpacePolicy>(input).is_err(),
                "{input}"
            );
        }
        let invalid = RuntimeSpacePolicy::Single {
            space: SpaceIdentity {
                tenant_id: "t".into(),
                space_id: " ".into(),
            },
        };
        assert!(invalid.validate().is_err());
        assert!(invalid.admit("t", "s").is_err());
    }
}

/// Fixed private file encoding, separate from the public Space wire identity.
pub mod stored {
    use super::*;
    #[derive(Serialize, Deserialize)]
    #[serde(
        remote = "SpaceIdentity",
        rename_all = "camelCase",
        deny_unknown_fields
    )]
    struct Identity {
        tenant_id: String,
        #[serde(rename = "scopeId")]
        space_id: String,
    }
    #[derive(Serialize, Deserialize)]
    struct Encoded(#[serde(with = "Identity")] SpaceIdentity);
    pub fn serialize_optional<S: serde::Serializer>(
        value: &Option<SpaceIdentity>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value.clone().map(Encoded).serialize(serializer)
    }
    pub fn deserialize_optional<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<SpaceIdentity>, D::Error> {
        Option::<Encoded>::deserialize(deserializer).map(|value| value.map(|value| value.0))
    }
    pub fn serialize<S: serde::Serializer>(
        value: &SpaceIdentity,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        Identity::serialize(value, serializer)
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<SpaceIdentity, D::Error> {
        Identity::deserialize(deserializer)
    }
}
