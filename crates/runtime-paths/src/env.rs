use serde::{Deserialize, Serialize};
use std::{path::Path, str::FromStr};

/// A local runtime environment, independent of build profile and cloud destination.
/// Only an absent wire field defaults to production; invalid values fail decoding.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeEnv {
    #[default]
    Prod,
    Dev,
    E2e,
}

impl RuntimeEnv {
    pub fn current() -> Self {
        crate::runtime_env()
    }
    pub fn is_prod(&self) -> bool {
        *self == Self::Prod
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Prod => "prod",
            Self::Dev => "dev",
            Self::E2e => "e2e",
        }
    }

    pub fn from_optional(value: Option<&str>) -> anyhow::Result<Self> {
        value.map(Self::from_str).unwrap_or(Ok(Self::Prod))
    }

    pub fn from_txt(value: Option<Option<&[u8]>>) -> anyhow::Result<Self> {
        match value {
            None => Ok(Self::Prod),
            Some(Some(value)) => Self::from_str(std::str::from_utf8(value)?),
            Some(None) => anyhow::bail!("runtime environment TXT property has no value"),
        }
    }

    /// Production names stay unchanged so existing peers continue to interoperate.
    pub fn mdns_service_type(self, component: &str) -> &'static str {
        match (self, component) {
            (Self::Prod, "host") => "_mhome-host._tcp.local.",
            (Self::Prod, "hub") => "_mhome-hub._tcp.local.",
            (Self::Prod, "node") => "_mhome-node._tcp.local.",
            (Self::Dev, "host") => "_mhome-host-dev._tcp.local.",
            (Self::Dev, "hub") => "_mhome-hub-dev._tcp.local.",
            (Self::Dev, "node") => "_mhome-node-dev._tcp.local.",
            (Self::E2e, "host") => "_mhome-host-e2e._tcp.local.",
            (Self::E2e, "hub") => "_mhome-hub-e2e._tcp.local.",
            (Self::E2e, "node") => "_mhome-node-e2e._tcp.local.",
            _ => panic!("unsupported runtime discovery component"),
        }
    }

    pub fn insert_mdns_property(self, properties: &mut impl Extend<(String, String)>) {
        if !self.is_prod() {
            properties.extend([("env".into(), self.as_str().into())]);
        }
    }

    /// Runtime management must not fall back to a cloud route lacking an env proof.
    /// Cloud-owned plugin catalogs and ordinary business requests are unaffected.
    pub fn requires_direct_management(self, target: &str) -> bool {
        !self.is_prod()
            && (target.starts_with("/app/system/hosts/")
                || (target.starts_with("/app/plugin/") && target != "/app/plugin/catalog/list"))
    }
}

impl FromStr for RuntimeEnv {
    type Err = anyhow::Error;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "prod" => Ok(Self::Prod),
            "dev" => Ok(Self::Dev),
            "e2e" => Ok(Self::E2e),
            _ => anyhow::bail!("invalid runtime environment"),
        }
    }
}

pub fn env_for_workdir(workdir: &Path, runtime_root: &Path) -> anyhow::Result<RuntimeEnv> {
    let root = runtime_root
        .canonicalize()
        .unwrap_or_else(|_| runtime_root.to_path_buf());
    let resolved = workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf());
    if let Some(name) = workdir.file_name().filter(|n| *n == "dev" || *n == "e2e") {
        if workdir
            .parent()
            .map(|p| p.canonicalize().unwrap_or_else(|_| p.to_path_buf()))
            .as_ref()
            == Some(&root)
        {
            anyhow::ensure!(
                resolved == root.join(name),
                "Dev/E2E workdir must not redirect to another environment"
            );
        }
    }
    if resolved == root.join("dev") {
        Ok(RuntimeEnv::Dev)
    } else if resolved == root.join("e2e") {
        Ok(RuntimeEnv::E2e)
    } else {
        Ok(RuntimeEnv::Prod)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_absence_defaults_to_prod() {
        assert_eq!(RuntimeEnv::from_optional(None).unwrap(), RuntimeEnv::Prod);
        for invalid in ["", "DEV", " dev", "test"] {
            assert!(RuntimeEnv::from_optional(Some(invalid)).is_err());
        }
        assert_eq!(
            RuntimeEnv::from_optional(Some("dev")).unwrap(),
            RuntimeEnv::Dev
        );
        assert!(serde_json::from_str::<RuntimeEnv>("null").is_err());
        assert_eq!(RuntimeEnv::from_txt(None).unwrap(), RuntimeEnv::Prod);
        assert_eq!(
            RuntimeEnv::from_txt(Some(Some(b"dev"))).unwrap(),
            RuntimeEnv::Dev
        );
        for value in [
            None,
            Some(b"".as_slice()),
            Some(b"DEV".as_slice()),
            Some(&[255][..]),
        ] {
            assert!(RuntimeEnv::from_txt(Some(value)).is_err());
        }
    }
    #[test]
    fn standard_roots_and_custom_directories() {
        let root = Path::new("/tmp/env-test/.meow");
        for env in [RuntimeEnv::Prod, RuntimeEnv::Dev, RuntimeEnv::E2e] {
            let dir = if env.is_prod() {
                root.to_path_buf()
            } else {
                root.join(env.as_str())
            };
            assert_eq!(env_for_workdir(&dir, root).unwrap(), env);
        }
        assert_eq!(
            env_for_workdir(Path::new("/tmp/custom/dev"), root).unwrap(),
            RuntimeEnv::Prod
        );
    }
    #[test]
    fn production_routes_and_discovery_are_compatible() {
        for component in ["host", "hub", "node"] {
            assert_ne!(
                RuntimeEnv::Prod.mdns_service_type(component),
                RuntimeEnv::Dev.mdns_service_type(component)
            );
            assert_ne!(
                RuntimeEnv::Dev.mdns_service_type(component),
                RuntimeEnv::E2e.mdns_service_type(component)
            );
        }
        assert!(!RuntimeEnv::Prod.requires_direct_management("/app/system/hosts/runtime"));
        assert!(RuntimeEnv::Dev.requires_direct_management("/app/system/hosts/runtime"));
        assert!(RuntimeEnv::E2e.requires_direct_management("/app/plugin/enabled/list"));
        assert!(!RuntimeEnv::Dev.requires_direct_management("/app/plugin/catalog/list"));
        assert!(!RuntimeEnv::Dev.requires_direct_management("/app/person/list"));
    }
}
