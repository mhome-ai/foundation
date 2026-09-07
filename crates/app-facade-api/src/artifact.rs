//! Public App Facade entry points for the shared Artifact protocol.

pub use artifact_api::{ResolveArtifactRequest, ResolveArtifactResponse};

/// Resolve an Artifact through the MeowLink-facing App Facade.
pub const RESOLVE_TARGET: &str = "/app/artifact/resolve";

pub const REQUEST_TARGETS: &[&str] = &[RESOLVE_TARGET];
