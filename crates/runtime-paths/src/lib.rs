mod endpoint;
#[cfg(unix)]
mod unix_process;
#[cfg(windows)]
mod windows_process;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub use endpoint::{
    meowclient_endpoint_label_for_workdir, meowclient_named_pipe_name_for_workdir,
    workdir_identity_hash,
};
#[cfg(unix)]
pub use unix_process::{
    clear_stale_unix_socket, is_pid_alive, process_executable_path, unix_peer_pid,
};
#[cfg(windows)]
pub use windows_process::{is_pid_alive, process_executable_path};

/// Compare process image paths, preferring canonical forms when available.
pub fn paths_match(actual: &Path, expected: &Path) -> bool {
    match (actual.canonicalize(), expected.canonicalize()) {
        (Ok(actual), Ok(expected)) => actual == expected,
        _ => actual == expected,
    }
}

/// True when `pid` is alive and its executable matches `expected_binary`.
pub fn pid_matches_executable(pid: u32, expected_binary: &Path) -> bool {
    if !is_pid_alive(pid) {
        return false;
    }
    match process_executable_path(pid) {
        Some(path) => paths_match(&path, expected_binary),
        None => false,
    }
}

/// If `pid_file` names a live process whose executable matches `expected_binary`,
/// returns its pid. Missing files return `Ok(None)`. Dead processes have their pid
/// file removed and return `Ok(None)`. Live processes with a different executable
/// are left untouched and return `Ok(None)`.
pub fn owned_daemon_pid_from_file(
    pid_file: &Path,
    expected_binary: &Path,
) -> Result<Option<u32>, String> {
    let raw = match fs::read_to_string(pid_file) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "failed to read daemon pid file {}: {}",
                pid_file.display(),
                err
            ))
        }
    };
    let pid = raw
        .trim()
        .parse::<u32>()
        .map_err(|err| format!("invalid daemon pid in {}: {}", pid_file.display(), err))?;

    if !is_pid_alive(pid) {
        match fs::remove_file(pid_file) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "failed to remove stale daemon pid file {}: {}",
                    pid_file.display(),
                    err
                ))
            }
        }
        return Ok(None);
    }
    if !pid_matches_executable(pid, expected_binary) {
        return Ok(None);
    }
    Ok(Some(pid))
}

/// Convenience wrapper: verify the default meowclient pid file against `expected_binary`.
pub fn owned_meowclient_pid(expected_binary: &Path) -> Result<Option<u32>, String> {
    owned_daemon_pid_from_file(&runtime_paths().meowclient_pid_file(), expected_binary)
}

static RUNTIME_WORKDIR: OnceLock<PathBuf> = OnceLock::new();

#[derive(Debug, Clone)]
pub struct RuntimePaths {
    workdir: PathBuf,
}

impl RuntimePaths {
    pub fn new(workdir: PathBuf) -> Self {
        Self { workdir }
    }

    pub fn for_workdir(workdir: impl AsRef<Path>) -> Self {
        Self::new(workdir.as_ref().to_path_buf())
    }

    pub fn workdir(&self) -> &Path {
        &self.workdir
    }

    pub fn system_root(&self) -> PathBuf {
        self.workdir.join("system")
    }

    pub fn services_root(&self) -> PathBuf {
        self.service_packages_root()
    }

    pub fn packages_root(&self) -> PathBuf {
        self.workdir.join("packages")
    }

    pub fn service_packages_root(&self) -> PathBuf {
        self.packages_root().join("services")
    }

    pub fn service_package_dir(&self, service_id: &str) -> PathBuf {
        self.service_packages_root().join(service_id)
    }

    pub fn config_root(&self) -> PathBuf {
        self.workdir.join("config")
    }

    pub fn run_root(&self) -> PathBuf {
        self.workdir.join("run")
    }

    pub fn log_root(&self) -> PathBuf {
        self.workdir.join("log")
    }

    pub fn data_root(&self) -> PathBuf {
        self.workdir.join("data")
    }

    pub fn state_root(&self) -> PathBuf {
        self.workdir.join("state")
    }

    pub fn downloads_root(&self) -> PathBuf {
        self.workdir.join("downloads")
    }

    pub fn tools_root(&self) -> PathBuf {
        self.workdir.join("tools")
    }

    pub fn downloads_tools_root(&self) -> PathBuf {
        self.downloads_root().join("tools")
    }

    pub fn artifacts_root(&self) -> PathBuf {
        self.workdir.join("artifacts")
    }

    pub fn component_config_path(&self, component_name: &str) -> PathBuf {
        self.config_root().join(format!("{component_name}.yaml"))
    }

    pub fn component_run_dir(&self, component_name: &str) -> PathBuf {
        self.run_root().join(component_name)
    }

    pub fn component_data_dir(&self, component_name: &str) -> PathBuf {
        self.data_root().join(component_name)
    }

    pub fn component_state_dir(&self, component_name: &str) -> PathBuf {
        self.state_root().join(component_name)
    }

    pub fn component_system_dir(&self, component_name: &str) -> PathBuf {
        self.system_root().join(component_name)
    }

    pub fn component_log_path(&self, component_name: &str) -> PathBuf {
        self.log_root().join(format!("{component_name}.log"))
    }

    pub fn service_data_dir(&self, service_id: &str) -> PathBuf {
        self.data_root().join("services").join(service_id)
    }

    pub fn service_run_dir(&self, service_id: &str) -> PathBuf {
        self.run_root().join("services").join(service_id)
    }

    pub fn service_log_path(&self, service_id: &str) -> PathBuf {
        self.log_root()
            .join(format!("{}.log", normalized_service_log_name(service_id)))
    }

    pub fn identity_file_path(&self) -> PathBuf {
        self.system_root().join("device_identity.json")
    }

    pub fn meowclient_socket_path(&self) -> PathBuf {
        self.component_run_dir("meowclient").join("meowd.sock")
    }

    pub fn meowclient_pid_file(&self) -> PathBuf {
        self.component_run_dir("meowclient").join("meowd.pid")
    }

    pub fn meowclient_ipc_token_file(&self) -> PathBuf {
        self.component_run_dir("meowclient").join("meowd.ipc.token")
    }

    pub fn meowclient_named_pipe_name(&self) -> String {
        meowclient_named_pipe_name_for_workdir(&self.workdir)
    }

    pub fn meowclient_endpoint_label(&self) -> String {
        meowclient_endpoint_label_for_workdir(&self.workdir)
    }
}

fn normalized_service_log_name(service_id: &str) -> String {
    match service_id {
        "meowcore" | "meow-core" => "meow-core".to_string(),
        value if value.starts_with("node-") => value.to_string(),
        value => format!("node-{value}"),
    }
}

pub fn set_runtime_workdir(workdir: PathBuf) -> anyhow::Result<()> {
    match RUNTIME_WORKDIR.get() {
        Some(existing) if existing == &workdir => Ok(()),
        Some(existing) => Err(anyhow::anyhow!(
            "runtime workdir already set to {}, cannot change to {}",
            existing.display(),
            workdir.display()
        )),
        None => RUNTIME_WORKDIR
            .set(workdir)
            .map_err(|_| anyhow::anyhow!("failed to set runtime workdir")),
    }
}

pub fn runtime_workdir() -> PathBuf {
    RUNTIME_WORKDIR
        .get()
        .expect("runtime workdir is not initialized at process startup")
        .clone()
}

pub fn runtime_paths() -> RuntimePaths {
    RuntimePaths::new(runtime_workdir())
}

pub fn resolve_workdir(dev: bool) -> PathBuf {
    let root = default_runtime_root().expect("failed to resolve default runtime workdir");
    if dev {
        root.join("dev")
    } else {
        root
    }
}

pub fn default_runtime_root() -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("LOCALAPPDATA is not set"))?;
        return Ok(PathBuf::from(local_app_data).join(".meow"));
    }

    #[cfg(not(windows))]
    {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("HOME is not set"))?;
        Ok(home.join(".meow"))
    }
}

#[cfg(not(windows))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_log_paths_use_standard_log_names() {
        let paths = RuntimePaths::for_workdir("/tmp/meow-runtime-test");

        assert_eq!(
            paths.service_log_path("camera"),
            PathBuf::from("/tmp/meow-runtime-test/log/node-camera.log")
        );
        assert_eq!(
            paths.service_log_path("node-camera"),
            PathBuf::from("/tmp/meow-runtime-test/log/node-camera.log")
        );
        assert_eq!(
            paths.service_log_path("meowcore"),
            PathBuf::from("/tmp/meow-runtime-test/log/meow-core.log")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn default_runtime_root_uses_dot_meow_under_home() {
        let root = default_runtime_root().unwrap();
        assert_eq!(
            root.file_name().and_then(|value| value.to_str()),
            Some(".meow")
        );
    }
}
