use anyhow::{bail, Context};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
#[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
use machineid_rs::{Encryption, HWIDComponent, IdBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
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
    if let Ok(raw) = fs::read_to_string(path) {
        let parsed: MachineIdentity =
            serde_json::from_str(&raw).context("failed to decode shared device identity")?;
        if !parsed.machine_id.is_empty() && !parsed.host_id.is_empty() {
            return Ok(parsed);
        }
    }

    let identity = derive_identity_from_raw_machine_id(&build_machine_id())?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create identity directory")?;
    }
    let raw = serde_json::to_string_pretty(&identity)?;
    fs::write(path, raw).context("failed to persist shared device identity")?;
    Ok(identity)
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
