//! Host-owned service intent and local admission coordination. Business databases
//! remain owned by services; the Host asks them for usage while admission is closed.
use crate::secure_file::{read_snapshot, write_private_export};
use anyhow::{Context, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub const CONTROL_DIR_ENV: &str = "MEOW_HOST_SERVICE_CONTROL_DIR";
pub const SERVICE_ID_ENV: &str = "MEOW_HOST_SERVICE_ID";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceIntent {
    pub enabled: bool,
}

/// All durable bindings, including offline/incomplete bindings, and unfinished
/// local cleanup. A failed query must never be converted to an empty usage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServiceUsage {
    pub space_count: usize,
    pub cleanup_pending: bool,
}
impl ServiceUsage {
    pub fn ensure_unused(&self) -> Result<()> {
        anyhow::ensure!(self.space_count == 0 && !self.cleanup_pending,
            "Service is still used by {} Space binding(s) or has unfinished cleanup; disconnect its Spaces first", self.space_count);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ServiceControl {
    path: PathBuf,
}
pub struct AdmissionGuard {
    _file: File,
}
pub struct ControlGuard {
    _file: File,
    control: ServiceControl,
}

// Explicit unlock also releases the lock if another thread briefly inherited
// the descriptor while spawning a child (before close-on-exec runs).
impl Drop for AdmissionGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self._file);
    }
}
impl Drop for ControlGuard {
    fn drop(&mut self) {
        let _ = FileExt::unlock(&self._file);
    }
}

impl ServiceControl {
    pub fn new(directory: &Path, service_id: &str) -> Result<Self> {
        anyhow::ensure!(
            directory.is_absolute(),
            "Host service control directory must be absolute"
        );
        anyhow::ensure!(
            !service_id.is_empty()
                && service_id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "Invalid service ID"
        );
        Ok(Self {
            path: directory.join(format!("{service_id}.json")),
        })
    }

    /// Absence of both variables explicitly denotes a standalone runtime. Host
    /// launchers always provide both; a partial/malformed context fails closed.
    pub fn from_environment() -> Result<Option<Self>> {
        match (
            std::env::var_os(CONTROL_DIR_ENV),
            std::env::var_os(SERVICE_ID_ENV),
        ) {
            (None, None) => Ok(None),
            (Some(dir), Some(id)) => Ok(Some(Self::new(
                Path::new(&dir),
                id.to_str().context("Invalid service ID encoding")?,
            )?)),
            _ => anyhow::bail!("Incomplete Host service control context"),
        }
    }

    /// Only Host calls this, before it launches a managed service. Existing and
    /// corrupt records are never overwritten with platform defaults.
    pub fn initialize(&self, enabled: bool) -> Result<ServiceIntent> {
        match fs::symlink_metadata(&self.path) {
            Ok(_) => return self.read(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
        let parent = self.path.parent().context("Missing control directory")?;
        fs::create_dir_all(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(parent, fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(windows)]
        crate::windows_security::apply_local_user_dacl(parent).map_err(anyhow::Error::msg)?;
        let file = self.lock_file(true)?;
        FileExt::try_lock_exclusive(&file).context("Service admission is busy; retry")?;
        let _guard = ControlGuard {
            _file: file,
            control: self.clone(),
        };
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                write_private_export(&self.path, &serde_json::to_vec(&ServiceIntent { enabled })?)?;
            }
            Err(e) => return Err(e.into()),
            Ok(_) => {}
        }
        self.read()
    }

    pub fn read(&self) -> Result<ServiceIntent> {
        read_snapshot(&self.path).context("Host service control state is unavailable")
    }

    pub fn admit(&self) -> Result<AdmissionGuard> {
        let file = self.lock_file(false)?;
        FileExt::try_lock_shared(&file).context("Service configuration is busy; retry")?;
        let guard = AdmissionGuard { _file: file };
        anyhow::ensure!(self.read()?.enabled, "Service is disabled on this Host");
        Ok(guard)
    }

    pub fn try_control(&self) -> Result<ControlGuard> {
        let file = self.lock_file(false)?;
        FileExt::try_lock_exclusive(&file).context("Service admission is busy; retry")?;
        let guard = ControlGuard {
            _file: file,
            control: self.clone(),
        };
        self.read()?;
        Ok(guard)
    }

    fn lock_file(&self, create: bool) -> Result<File> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .write(true)
            .create(create)
            .truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        let file = options.open(self.path.with_extension("lock"))?;
        anyhow::ensure!(file.metadata()?.is_file(), "Invalid service control lock");
        Ok(file)
    }
}

impl ControlGuard {
    pub fn read(&self) -> Result<ServiceIntent> {
        self.control.read()
    }
    /// Host owns this operation; services never write service intent.
    pub fn set_enabled(&self, enabled: bool) -> Result<()> {
        write_private_export(
            &self.control.path,
            &serde_json::to_vec(&ServiceIntent { enabled })?,
        )
    }
}

/// Hold the returned permit throughout activation and durable binding creation.
pub fn admit_managed_service() -> Result<Option<AdmissionGuard>> {
    ServiceControl::from_environment()?
        .map(|control| control.admit())
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn intent_survives_reopen_and_failed_queries_do_not_change_it() {
        let dir = tempfile::tempdir().unwrap();
        let control = ServiceControl::new(dir.path(), "camera").unwrap();
        assert!(control.read().is_err());
        control.initialize(true).unwrap();
        let admission = control.admit().unwrap();
        assert!(control.try_control().is_err());
        drop(admission);
        let guard = control.try_control().unwrap();
        assert!(control.admit().is_err());
        assert!(ServiceUsage {
            space_count: 1,
            cleanup_pending: false
        }
        .ensure_unused()
        .is_err());
        assert!(ServiceUsage {
            space_count: 0,
            cleanup_pending: true
        }
        .ensure_unused()
        .is_err());
        drop(guard); // Query failure/occupied: no write.
        assert!(control.read().unwrap().enabled);
        control.try_control().unwrap().set_enabled(false).unwrap();
        assert!(!control.initialize(true).unwrap().enabled);
        assert!(ServiceControl::new(dir.path(), "camera")
            .unwrap()
            .admit()
            .is_err());
        control.try_control().unwrap().set_enabled(true).unwrap();
        assert!(control.admit().is_ok());
        fs::write(&control.path, "broken").unwrap();
        assert!(control.initialize(true).is_err());
        assert!(control.admit().is_err());
    }

    #[test]
    fn independent_services_do_not_block_each_other() {
        let dir = tempfile::tempdir().unwrap();
        let camera = ServiceControl::new(dir.path(), "camera").unwrap();
        let core = ServiceControl::new(dir.path(), "meowcore").unwrap();
        camera.initialize(true).unwrap();
        core.initialize(true).unwrap();
        let _camera_admission = camera.admit().unwrap();
        core.try_control().unwrap().set_enabled(false).unwrap();
        assert!(camera.read().unwrap().enabled);
        assert!(camera.try_control().is_err());
        assert!(core.admit().is_err());
    }

    #[test]
    fn process_exit_releases_admission_but_preserves_intent() {
        const CHILD: &str = "MEOW_SERVICE_CONTROL_TEST_DIR";
        if let Some(dir) = std::env::var_os(CHILD) {
            let control = ServiceControl::new(Path::new(&dir), "camera").unwrap();
            let _guard = control.admit().unwrap();
            fs::write(Path::new(&dir).join("ready"), "").unwrap();
            loop {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
        }
        let dir = tempfile::tempdir().unwrap();
        let control = ServiceControl::new(dir.path(), "camera").unwrap();
        control.initialize(true).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "service_control::tests::process_exit_releases_admission_but_preserves_intent",
            ])
            .env(CHILD, dir.path())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !dir.path().join("ready").exists() {
            if std::time::Instant::now() > deadline {
                let _ = child.kill();
                panic!("child did not acquire admission");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(control.try_control().is_err());
        child.kill().unwrap();
        child.wait().unwrap();
        control.try_control().unwrap().set_enabled(false).unwrap();
        assert!(!control.read().unwrap().enabled);
        assert!(control.admit().is_err());
    }
}
