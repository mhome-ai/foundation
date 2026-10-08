use anyhow::{bail, Context};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineIdentity {
    pub machine_id: String,
    pub host_id: String,
    pub device_suffix: String,
    pub version: u32,
}

pub fn get_or_create_identity_at(path: &Path) -> anyhow::Result<MachineIdentity> {
    identity_at(
        path,
        &runtime_paths::default_runtime_root()?,
        build_machine_id,
    )
}

fn identity_at(
    path: &Path,
    runtime_root: &Path,
    build_id: impl FnOnce() -> String,
) -> anyhow::Result<MachineIdentity> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).context("failed to create identity directory")?;
    // Host, Client, Core and Nodes can start concurrently. Keep the lock file in
    // place: removing it would let another process lock a different inode.
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path.with_extension("lock"))?;
    fs2::FileExt::lock_exclusive(&lock).context("failed to lock shared device identity")?;
    let suffix = environment_suffix(path, runtime_root)?;
    let (mut identity, mut changed) = match fs::read_to_string(path) {
        Ok(raw) => {
            let parsed: MachineIdentity =
                serde_json::from_str(&raw).context("failed to decode shared device identity")?;
            anyhow::ensure!(
                parsed.version == 1
                    && !parsed.machine_id.trim().is_empty()
                    && !parsed.host_id.is_empty(),
                "invalid shared device identity"
            );
            (parsed, false)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (derive_identity_from_raw_machine_id(&build_id())?, true)
        }
        Err(error) => return Err(error).context("failed to read shared device identity"),
    };
    if !suffix.is_empty() {
        let base = hash_string(identity.machine_id.trim());
        let scoped = format!("{base}{suffix}");
        anyhow::ensure!(
            identity.host_id == base || identity.host_id == scoped,
            "device identity belongs to a different environment; reconfigure this installation"
        );
        if identity.host_id != scoped {
            identity.host_id = scoped;
            changed = true;
        }
    }
    if changed {
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        file.write_all(serde_json::to_string_pretty(&identity)?.as_bytes())?;
        file.as_file().sync_all()?;
        file.persist(path)
            .context("failed to persist shared device identity")?;
    }
    Ok(identity)
}

fn environment_suffix(path: &Path, runtime_root: &Path) -> anyhow::Result<&'static str> {
    // These are the public Harness workdirs, regardless of release/debug build
    // or cloud destination. An unrelated directory named "dev" is not Dev.
    let Some(system) = path
        .parent()
        .filter(|p| p.file_name().is_some_and(|n| n == "system"))
    else {
        return Ok("");
    };
    if path.file_name().is_none_or(|n| n != "device_identity.json") {
        return Ok("");
    }
    let workdir = system.parent().unwrap_or(Path::new("."));
    let Ok(root) = runtime_root.canonicalize() else {
        return Ok("");
    };
    let resolved = workdir
        .canonicalize()
        .context("failed to resolve identity workdir")?;
    if let Some(name) = workdir.file_name().filter(|n| *n == "dev" || *n == "e2e") {
        if workdir
            .parent()
            .and_then(|p| p.canonicalize().ok())
            .as_ref()
            == Some(&root)
        {
            anyhow::ensure!(
                resolved == root.join(name),
                "Dev/E2E workdir must not redirect to another environment"
            );
        }
    }
    let workdir = resolved;
    if workdir == root.join("dev") {
        return Ok("-dev");
    }
    if workdir == root.join("e2e") {
        return Ok("-e2e");
    }
    Ok("")
}

pub fn derive_identity_from_raw_machine_id(
    raw_machine_id: &str,
) -> anyhow::Result<MachineIdentity> {
    let machine_id = raw_machine_id.trim();
    if machine_id.is_empty() {
        bail!("raw machine id is required");
    }

    let host_id = hash_string(machine_id);
    Ok(MachineIdentity {
        machine_id: machine_id.to_string(),
        device_suffix: derive_device_suffix(&host_id),
        host_id,
        version: 1,
    })
}

pub fn get_host_type() -> String {
    #[cfg(target_os = "android")]
    return "android".to_string();

    #[cfg(target_os = "macos")]
    return "macos".to_string();

    #[cfg(target_os = "windows")]
    return "windows".to_string();

    #[cfg(target_os = "linux")]
    return "linux".to_string();

    #[cfg(not(any(
        target_os = "android",
        target_os = "macos",
        target_os = "windows",
        target_os = "linux"
    )))]
    return "desktop".to_string();
}

pub fn get_host_name_with_identity_at(identity_path: &Path) -> String {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = std::process::Command::new("scutil")
            .arg("--get")
            .arg("ComputerName")
            .output()
        {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    return trimmed.to_string();
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(hostname) = std::fs::read_to_string("/etc/hostname") {
            let trimmed = hostname.trim();
            if !trimmed.is_empty() {
                return capitalize(trimmed);
            }
        }

        if let Ok(output) = std::process::Command::new("hostname").output() {
            if let Ok(name) = String::from_utf8(output.stdout) {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    return capitalize(trimmed);
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(name) = std::env::var("COMPUTERNAME") {
            if !name.is_empty() {
                return capitalize(&name);
            }
        }
    }

    let suffix = get_or_create_identity_at(identity_path)
        .map(|identity| identity.device_suffix)
        .unwrap_or_else(|_| "NODE".to_string());
    format!("{}-{}", capitalize(&get_host_type()), suffix)
}

#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
fn build_machine_id() -> String {
    let mut builder = IdBuilder::new(Encryption::SHA256);
    builder
        .add_component(HWIDComponent::SystemID)
        .add_component(HWIDComponent::CPUID);

    match builder.build("mhome") {
        Ok(id) => id,
        Err(_) => uuid::Uuid::new_v4().to_string(),
    }
}

#[cfg(target_os = "android")]
fn build_machine_id() -> String {
    panic!("machine-identity must not generate machine id on Android; use CoreRuntimeContext.rawMachineId")
}

#[cfg(not(any(
    target_os = "android",
    target_os = "linux",
    target_os = "macos",
    target_os = "windows"
)))]
fn build_machine_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    URL_SAFE_NO_PAD.encode(result)
}

fn derive_device_suffix(host_id: &str) -> String {
    let suffix: String = host_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_uppercase())
        .take(4)
        .collect();
    if suffix.is_empty() {
        "NODE".to_string()
    } else {
        suffix
    }
}

fn capitalize(input: &str) -> String {
    let mut chars = input.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn load(path: &Path, root: &Path) -> MachineIdentity {
        identity_at(path, root, || "same-physical-machine".into()).unwrap()
    }

    #[test]
    fn environments_are_distinct_and_stable_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".meow");
        let base = derive_identity_from_raw_machine_id("same-physical-machine").unwrap();
        for (environment, suffix) in [("", ""), ("dev", "-dev"), ("e2e", "-e2e")] {
            let path = root.join(environment).join("system/device_identity.json");
            let first = load(&path, &root);
            assert_eq!(first.host_id, format!("{}{suffix}", base.host_id));
            assert_eq!(first.machine_id, base.machine_id);
            assert_eq!(first.device_suffix, base.device_suffix);
            let again =
                identity_at(&path, &root, || panic!("must use persisted identity")).unwrap();
            assert_eq!(again.host_id, first.host_id);
            let stored: MachineIdentity = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
            assert_eq!(stored.host_id, first.host_id);
        }
    }

    #[test]
    fn legacy_development_record_uses_saved_machine_id_once() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let path = root.join("dev/system/device_identity.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = derive_identity_from_raw_machine_id("saved-hardware-fallback").unwrap();
        fs::write(&path, serde_json::to_vec(&old).unwrap()).unwrap();
        let first = load(&path, root);
        assert_eq!(first.host_id, format!("{}-dev", old.host_id));
        assert_eq!(first.machine_id, old.machine_id);
        assert_eq!(load(&path, root).host_id, first.host_id);
    }

    #[test]
    fn production_record_is_not_rewritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("system/device_identity.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let old = derive_identity_from_raw_machine_id("existing-production-machine").unwrap();
        let bytes = serde_json::to_vec(&old).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert_eq!(load(&path, dir.path()).host_id, old.host_id);
        assert_eq!(fs::read(path).unwrap(), bytes);
    }

    #[test]
    fn custom_directory_named_dev_does_not_select_harness_environment() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(".meow");
        fs::create_dir_all(&root).unwrap();
        let path = dir.path().join("custom/dev/system/device_identity.json");
        assert_eq!(
            load(&path, &root).host_id,
            hash_string("same-physical-machine")
        );
    }

    #[test]
    fn corrupt_and_unreadable_records_are_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("system/device_identity.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        for bytes in [
            b"{broken".as_slice(),
            br#"{"machineId":"","hostId":"old","deviceSuffix":"OLD","version":1}"#,
        ] {
            fs::write(&path, bytes).unwrap();
            assert!(
                identity_at(&path, dir.path(), || panic!("must not replace identity")).is_err()
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert!(identity_at(&path, dir.path(), || panic!("must not replace identity")).is_err());
        assert!(path.is_dir());
    }

    #[test]
    fn copied_identity_from_another_environment_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let dev = dir.path().join("dev/system/device_identity.json");
        let e2e = dir.path().join("e2e/system/device_identity.json");
        load(&dev, dir.path());
        fs::create_dir_all(e2e.parent().unwrap()).unwrap();
        fs::copy(&dev, &e2e).unwrap();
        assert!(identity_at(&e2e, dir.path(), || panic!("must not replace identity")).is_err());
    }

    #[test]
    fn concurrent_first_start_generates_one_identity() {
        let dir = tempfile::tempdir().unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let root = dir.path().to_path_buf();
                let count = count.clone();
                std::thread::spawn(move || {
                    identity_at(&root.join("dev/system/device_identity.json"), &root, || {
                        format!("fallback-{}", count.fetch_add(1, Ordering::SeqCst))
                    })
                    .unwrap()
                    .host_id
                })
            })
            .collect();
        let ids: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
        assert!(ids.iter().all(|id| id == &ids[0]));
        assert_eq!(count.load(Ordering::SeqCst), 1);
    }

    #[cfg(unix)]
    #[test]
    fn development_symlink_cannot_reuse_production_storage() {
        let dir = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(dir.path(), dir.path().join("dev")).unwrap();
        let path = dir.path().join("dev/system/device_identity.json");
        assert!(identity_at(&path, dir.path(), || panic!("must not create identity")).is_err());
        assert!(!path.exists());
    }
}
