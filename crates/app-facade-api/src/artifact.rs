//! Public App Facade entry points for the shared Artifact protocol.

pub use artifact_api::{
    CancelArtifactUploadRequest, CancelArtifactUploadResponse, CompleteArtifactUploadRequest,
    CompleteArtifactUploadResponse, DeleteArtifactRequest, ImportArtifactRequest,
    PutArtifactRequest, PutArtifactResponse, ResolveArtifactRequest, ResolveArtifactResponse,
    StartArtifactUploadRequest, StartArtifactUploadResponse,
};

/// Resolve an Artifact through the MeowLink-facing App Facade.
pub const RESOLVE_TARGET: &str = "/app/artifact/resolve";

/// Resolve a public HTTPS delivery URL through the cloud without replicating content.
pub const EXTERNAL_RESOLVE_TARGET: &str = "/app/artifact/external/resolve";

pub const PUT_TARGET: &str = "/app/artifact/put";
pub const IMPORT_TARGET: &str = "/app/artifact/import";

pub const DELETE_TARGET: &str = "/app/artifact/delete";

pub const UPLOAD_START_TARGET: &str = "/app/artifact/upload/start";
pub const UPLOAD_COMPLETE_TARGET: &str = "/app/artifact/upload/complete";
pub const UPLOAD_CANCEL_TARGET: &str = "/app/artifact/upload/cancel";
pub const REQUEST_TARGETS: &[&str] = &[
    RESOLVE_TARGET,
    EXTERNAL_RESOLVE_TARGET,
    PUT_TARGET,
    IMPORT_TARGET,
    DELETE_TARGET,
    UPLOAD_START_TARGET,
    UPLOAD_COMPLETE_TARGET,
    UPLOAD_CANCEL_TARGET,
];
