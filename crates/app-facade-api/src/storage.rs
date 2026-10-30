//! App entrypoints reuse the canonical Storage domain and Core session contracts.
pub use core_api::ObjectStorageSession;
pub use storage_api::*;
pub const PREFIX: &str = "/app/object-storage/";
