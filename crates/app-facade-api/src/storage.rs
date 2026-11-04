//! App entrypoints reuse the canonical Storage domain and Core session contracts.
pub use core_api::ObjectStorageSession;
pub use storage_api::*;
pub const PREFIX: &str = "/app/object-storage/";

/// Space-member subtree capacity management; independent of content ownership.
pub const SET_FOLDER_LIMIT_TARGET: &str = "/app/object-storage/folders/set-limit";
