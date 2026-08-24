use std::path::PathBuf;

use sha2::{Digest, Sha256};

/// Stable workdir identity for endpoint names (pipe / labels).
///
/// Uses SHA-256 of the canonical workdir path so the value does not change
/// across Rust/std hasher versions. Returns the first 16 hex chars.
pub fn workdir_identity_hash(workdir: &std::path::Path) -> String {
    let normalized = normalize_workdir_for_hash(workdir);
    let bytes = normalized.to_string_lossy();
    let digest = Sha256::digest(bytes.as_bytes());
    hex_encode_prefix(&digest, 16)
}

fn hex_encode_prefix(bytes: &[u8], chars: usize) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let nibble_count = chars.min(bytes.len() * 2);
    let mut out = String::with_capacity(nibble_count);
    for byte in bytes {
        if out.len() >= nibble_count {
            break;
        }
        out.push(HEX[(byte >> 4) as usize] as char);
        if out.len() >= nibble_count {
            break;
        }
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}

fn normalize_workdir_for_hash(workdir: &std::path::Path) -> PathBuf {
    workdir
        .canonicalize()
        .unwrap_or_else(|_| workdir.to_path_buf())
}

pub fn meowclient_named_pipe_name_for_workdir(workdir: &std::path::Path) -> String {
    format!(
        r"\\.\pipe\meowclient-meowd-{}",
        workdir_identity_hash(workdir)
    )
}

#[cfg(unix)]
pub fn meowclient_endpoint_label_for_workdir(workdir: &std::path::Path) -> String {
    crate::RuntimePaths::for_workdir(workdir)
        .meowclient_socket_path()
        .display()
        .to_string()
}

#[cfg(windows)]
pub fn meowclient_endpoint_label_for_workdir(workdir: &std::path::Path) -> String {
    meowclient_named_pipe_name_for_workdir(workdir)
}

#[cfg(not(any(unix, windows)))]
pub fn meowclient_endpoint_label_for_workdir(workdir: &std::path::Path) -> String {
    crate::RuntimePaths::for_workdir(workdir)
        .meowclient_socket_path()
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn identity_hash_is_stable_hex() {
        let hash = workdir_identity_hash(Path::new("/tmp/meow-workdir-test"));
        assert_eq!(hash.len(), 16);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            hash,
            workdir_identity_hash(Path::new("/tmp/meow-workdir-test"))
        );
    }
}
