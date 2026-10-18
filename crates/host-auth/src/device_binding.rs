//! One protected, atomically replaced binding shared by a fixed-Space Client
//! and its optional dependent Host. Only the Client coordinator uses Writer.
use crate::{secure_file::SecureFile, signatures::random_id};
use anyhow::{ensure, Context, Result};
pub use core_api::device_binding::*;
use core_api::{runtime_space::SpaceIdentity, RuntimeEnv};
use std::path::PathBuf;

#[derive(Clone)]
pub struct Reader {
    path: PathBuf,
}
impl Reader {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    pub fn read(&self) -> Result<DeviceBinding> {
        let value: DeviceBinding = crate::secure_file::read_snapshot(&self.path)?;
        value.validate().map_err(anyhow::Error::msg)?;
        Ok(value)
    }
    pub fn for_host(
        &self,
        host: &DeviceComponentIdentity,
        env: RuntimeEnv,
    ) -> Result<DeviceBinding> {
        ensure!(!self.reset_pending()?, "product reset is pending");
        let value = self.read()?;
        ensure!(
            value.env == env && value.host.as_ref() == Some(host),
            "dependent Host binding identity mismatch"
        );
        Ok(value)
    }
}

pub struct Writer {
    store: SecureFile<Option<DeviceBinding>>,
}
impl Writer {
    pub fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            store: SecureFile::new(path)?,
        })
    }
    pub fn reader(&self) -> Reader {
        Reader::new(self.store.path().into())
    }

    /// Called only for a new Client installation. A persisted Client which has
    /// already created this file must use Reader and fail if the file is missing.
    pub fn initialize(
        &self,
        env: RuntimeEnv,
        client: DeviceComponentIdentity,
        host: Option<DeviceComponentIdentity>,
    ) -> Result<DeviceBinding> {
        self.store.update(|saved| {
            if let Some(value) = saved {
                value.validate().map_err(anyhow::Error::msg)?;
                ensure!(
                    value.env == env && value.client == client && value.host == host,
                    "device installation identity changed; reset required"
                );
                return Ok(value.clone());
            }
            let value = DeviceBinding {
                version: DEVICE_BINDING_VERSION,
                env,
                client,
                host,
                generation: random_id(),
                space: None,
            };
            value.validate().map_err(anyhow::Error::msg)?;
            *saved = Some(value.clone());
            Ok(value)
        })
    }

    /// The caller verifies owner approval bound to these exact identities and
    /// generation before entering this transaction. Retrying the same Space is
    /// harmless; switching Space always requires factory reset.
    pub fn bind_verified(
        &self,
        expected: &DeviceBinding,
        space: SpaceIdentity,
    ) -> Result<DeviceBinding> {
        space.validate().map_err(anyhow::Error::msg)?;
        self.store.update(|saved| {
            let current = saved.as_mut().context("device binding missing")?;
            current.validate().map_err(anyhow::Error::msg)?;
            ensure!(
                same_installation(current, expected),
                "device binding changed during authorization"
            );
            ensure!(
                current.space.as_ref().is_none_or(|s| *s == space),
                "device belongs to another Space; reset required"
            );
            current.space = Some(space);
            Ok(current.clone())
        })
    }
}
fn same_installation(a: &DeviceBinding, b: &DeviceBinding) -> bool {
    a.version == b.version
        && a.env == b.env
        && a.client == b.client
        && a.host == b.host
        && a.generation == b.generation
}

#[cfg(test)]
mod tests {
    use super::*;
    fn id(id: &str) -> DeviceComponentIdentity {
        DeviceComponentIdentity {
            id: id.into(),
            key_id: format!("key:{id}"),
        }
    }
    fn space(id: &str) -> SpaceIdentity {
        SpaceIdentity {
            tenant_id: "tenant".into(),
            scope_id: id.into(),
        }
    }
    #[test]
    fn one_binding_controls_retry_reset_and_host_identity() {
        let dir = tempfile::tempdir().unwrap();
        let writer = Writer::new(dir.path().join("device-binding.json")).unwrap();
        assert!(writer.reader().read().is_err());
        let initial = writer
            .initialize(RuntimeEnv::Dev, id("client"), Some(id("host")))
            .unwrap();
        let bound = writer.bind_verified(&initial, space("home")).unwrap();
        assert_eq!(
            writer.bind_verified(&initial, space("home")).unwrap(),
            bound
        );
        assert!(writer.bind_verified(&initial, space("other")).is_err());
        assert_eq!(
            writer
                .reader()
                .for_host(&id("host"), RuntimeEnv::Dev)
                .unwrap(),
            bound
        );
        assert!(writer
            .reader()
            .for_host(&id("other-host"), RuntimeEnv::Dev)
            .is_err());
        assert!(writer
            .reader()
            .for_host(&id("host"), RuntimeEnv::Prod)
            .is_err());
        let journal = ResetJournal::for_binding(writer.store.path()).unwrap();
        let request = journal.request(RuntimeEnv::Dev, id("host"), false).unwrap();
        let reset = writer
            .replace_after_reset(
                &request,
                id("new-client"),
                DeviceComponentIdentity {
                    id: "host".into(),
                    key_id: "rotated-host-key".into(),
                },
            )
            .unwrap();
        journal.complete(&request.operation_id).unwrap();
        assert_ne!(reset.generation, initial.generation);
        assert!(reset.space.is_none());
        assert!(writer.bind_verified(&initial, space("home")).is_err());
        std::fs::write(writer.store.path(), b"broken").unwrap();
        assert!(writer.reader().read().is_err());
        assert!(writer
            .initialize(RuntimeEnv::Dev, id("client"), Some(id("host")))
            .is_err());
    }
    #[test]
    fn reset_intent_blocks_reads_and_recovers_a_damaged_binding_only_after_host_rotation() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device-binding.json");
        let writer = Writer::new(path.clone()).unwrap();
        let initial = writer
            .initialize(RuntimeEnv::Dev, id("client"), Some(id("host")))
            .unwrap();
        writer.bind_verified(&initial, space("home")).unwrap();
        let journal = ResetJournal::for_binding(&path).unwrap();
        let request = journal.request(RuntimeEnv::Dev, id("host"), false).unwrap();
        assert_eq!(
            journal.request(RuntimeEnv::Dev, id("host"), true).unwrap(),
            request
        );
        assert!(writer
            .reader()
            .for_host(&id("host"), RuntimeEnv::Dev)
            .is_err());
        assert!(journal.complete("wrong-operation").is_err());
        std::fs::write(&path, b"interrupted old data").unwrap();
        assert!(writer
            .replace_after_reset(&request, id("new-client"), id("host"))
            .is_err());
        let new_host = DeviceComponentIdentity {
            id: "host".into(),
            key_id: "rotated-key".into(),
        };
        let replaced = writer
            .replace_after_reset(&request, id("new-client"), new_host.clone())
            .unwrap();
        assert!(replaced.space.is_none());
        assert_ne!(replaced.generation, initial.generation);
        // Crash here: reopening still blocks every Host request until the Client
        // has cleared its credential DB and commits the completion marker.
        assert!(Reader::new(path.clone())
            .for_host(&new_host, RuntimeEnv::Dev)
            .is_err());
        assert_eq!(
            Writer::new(path.clone())
                .unwrap()
                .replace_after_reset(&request, id("new-client"), new_host.clone())
                .unwrap(),
            replaced
        );
        journal.complete(&request.operation_id).unwrap();
        assert_eq!(
            writer
                .reader()
                .for_host(&new_host, RuntimeEnv::Dev)
                .unwrap(),
            replaced
        );
        assert!(writer.bind_verified(&initial, space("home")).is_err());
        assert!(writer
            .replace_after_reset(&request, id("new-client"), new_host)
            .is_err());
    }

    #[test]
    fn simultaneous_first_bindings_cannot_select_two_spaces() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("device-binding.json");
        let initial = Writer::new(path.clone())
            .unwrap()
            .initialize(RuntimeEnv::Dev, id("client"), None)
            .unwrap();
        let tasks: Vec<_> = ["one", "two"]
            .into_iter()
            .map(|name| {
                let path = path.clone();
                let initial = initial.clone();
                std::thread::spawn(move || {
                    Writer::new(path)
                        .unwrap()
                        .bind_verified(&initial, space(name))
                })
            })
            .collect();
        assert_eq!(
            tasks
                .into_iter()
                .map(|t| t.join().unwrap().is_ok() as usize)
                .sum::<usize>(),
            1
        );
    }
}

/// One durable product-reset request, shared by both entrypoints. It contains no
/// credentials. The Client coordinator completes it after both components reset.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResetRequest {
    pub operation_id: String,
    pub env: RuntimeEnv,
    pub host: DeviceComponentIdentity,
    pub clear_wifi: bool,
}
pub struct ResetJournal {
    store: SecureFile<Option<ResetRequest>>,
}
impl ResetJournal {
    pub fn for_binding(binding: &std::path::Path) -> Result<Self> {
        Ok(Self {
            store: SecureFile::new(
                binding
                    .parent()
                    .context("binding parent missing")?
                    .join("device-reset.json"),
            )?,
        })
    }
    pub fn pending(&self) -> Result<Option<ResetRequest>> {
        self.store.read()
    }
    pub fn request(
        &self,
        env: RuntimeEnv,
        host: DeviceComponentIdentity,
        clear_wifi: bool,
    ) -> Result<ResetRequest> {
        host.validate().map_err(anyhow::Error::msg)?;
        self.store.update(|saved| {
            if let Some(request) = saved {
                return Ok(request.clone());
            }
            let request = ResetRequest {
                operation_id: random_id(),
                env,
                host,
                clear_wifi,
            };
            *saved = Some(request.clone());
            Ok(request)
        })
    }
    pub fn complete(&self, operation_id: &str) -> Result<()> {
        self.store.update(|saved| {
            ensure!(
                saved
                    .as_ref()
                    .is_some_and(|r| r.operation_id == operation_id),
                "product reset changed"
            );
            *saved = None;
            Ok(())
        })
    }
}
impl Reader {
    pub fn request_reset(&self, env: RuntimeEnv, host: DeviceComponentIdentity) -> Result<()> {
        ResetJournal::for_binding(&self.path)?.request(env, host, true)?;
        Ok(())
    }
    /// Nonblocking read: authorization never waits for a reset writer's lock.
    pub fn reset_pending(&self) -> Result<bool> {
        let path = self
            .path
            .parent()
            .context("binding parent missing")?
            .join("device-reset.json");
        match crate::secure_file::read_snapshot::<Option<ResetRequest>>(&path) {
            Ok(value) => Ok(value.is_some()),
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                Ok(false)
            }
            Err(error) => Err(error),
        }
    }
}
impl Writer {
    /// Startup-only recovery after an explicit product reset, while the Client
    /// process lock is held and Host children have stopped. Also recovers a
    /// missing/corrupt binding; ordinary startup and onboarding never overwrite it.
    pub fn replace_after_reset(
        &self,
        reset: &ResetRequest,
        client: DeviceComponentIdentity,
        host: DeviceComponentIdentity,
    ) -> Result<DeviceBinding> {
        ensure!(
            ResetJournal::for_binding(self.store.path())?
                .pending()?
                .as_ref()
                == Some(reset),
            "explicit reset request required"
        );
        ensure!(
            host != reset.host && host.id == reset.host.id,
            "Host has not completed its reset"
        );
        let value = DeviceBinding {
            version: DEVICE_BINDING_VERSION,
            env: reset.env,
            client,
            host: Some(host),
            generation: reset.operation_id.clone(),
            space: None,
        };
        value.validate().map_err(anyhow::Error::msg)?;
        crate::secure_file::write_private_export(self.store.path(), &serde_json::to_vec(&value)?)?;
        Ok(value)
    }
}
