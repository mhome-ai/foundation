//! Public App Facade entry points for the shared Artifact protocol.

pub use artifact_api::{
    ImportArtifactRequest, PutArtifactRequest, PutArtifactResponse, ResolveArtifactRequest,
    ResolveArtifactResponse,
};

/// Resolve an Artifact through the MeowLink-facing App Facade.
pub const RESOLVE_TARGET: &str = "/app/artifact/resolve";

pub const PUT_TARGET: &str = "/app/artifact/put";
pub const IMPORT_TARGET: &str = "/app/artifact/import";

pub const REQUEST_TARGETS: &[&str] = &[RESOLVE_TARGET, PUT_TARGET, IMPORT_TARGET];
