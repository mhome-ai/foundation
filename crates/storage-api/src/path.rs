//! Logical names are exact, case-sensitive UTF-8 paths, scoped to a Space.
//! The catalog maps them to private incarnations; never join them to a disk root.
use crate::{StorageError, StorageErrorCode};

pub fn validate_folder_id(id: &str) -> Result<(), StorageError> {
    let parts: Vec<_> = id.split('/').collect();
    let valid = id.len() <= 1024
        && parts.len() <= 16
        && parts.iter().all(|part| {
            !part.is_empty()
                && *part == part.trim()
                && *part != "."
                && *part != ".."
                && part.chars().count() <= 80
                && !part.chars().any(|c| c.is_control() || c == '\\')
        })
        && match parts.first().copied() {
            Some("artifact") => parts.len() == 1,
            Some("home") => parts.len() >= 2,
            Some("plugin") => parts.len() >= 4,
            Some("core") => parts.len() >= 3,
            _ => false,
        };
    if valid {
        Ok(())
    } else {
        Err(StorageError::new(StorageErrorCode::InvalidInput, "Invalid Folder path: use home/<path>, plugin/<name>/<instance>/<path>, or core/<module>/<path>; artifact is reserved"))
    }
}

/// Segment boundary comparison; names containing SQL wildcards are ordinary names.
pub fn is_folder_within(id: &str, parent: &str) -> bool {
    id == parent
        || id
            .strip_prefix(parent)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_are_exact_bounded_and_segment_scoped() {
        for id in [
            "artifact",
            "home/照片",
            "home/a_b%/c",
            "plugin/camera/one/recordings",
            "core/cache/files",
        ] {
            assert!(validate_folder_id(id).is_ok(), "{id}");
        }
        for id in [
            "",
            "home",
            "home/",
            "/home/a",
            "home/../b",
            "home/a//b",
            "home/ a",
            "home/a\\b",
            "artifact/a",
            "plugin/camera/a",
            "other/a",
        ] {
            assert!(validate_folder_id(id).is_err(), "{id}");
        }
        assert!(is_folder_within("home/a/b", "home/a"));
        assert!(!is_folder_within("home/ab", "home/a"));
        assert!(!is_folder_within("home/A", "home/a"));
    }
}
