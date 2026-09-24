//! Transport-neutral Person contracts. No camera, storage or inference dependencies.
pub mod gallery;
pub mod learning;
pub mod management;
pub mod model;
pub use gallery::*;
pub use learning::*;
pub use model::*;
pub const PROTOCOL_VERSION: &str = "person.v1";
pub const DESCRIBE_TARGET: &str = "/person/describe";
pub const GALLERY_TARGET: &str = "/person/gallery/sync";
pub const SUBMIT_TARGET: &str = "/person/samples/submit";
pub const ACK_TARGET: &str = "/person/gallery/ack";
pub const IMPORT_TARGET: &str = "/person/legacy/import";
pub const MAX_SAMPLES_PER_BATCH: usize = 16;
pub const MAX_VECTOR_DIMENSIONS: usize = 4096;
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;
pub const GALLERY_LEASE_MS: u64 = 300_000;
pub const MATCH_POLICY: &str = "cosine-prototype.v1";

pub const IMPORT_FINISH_TARGET: &str = "/person/legacy/finish";

/// Live evidence is not a durable offline queue. Receipts outlive this acceptance window.
pub const MAX_EVIDENCE_AGE_MS: u64 = 24 * 60 * 60 * 1000;

pub const IMPORT_OFFER_TARGET: &str = "/person/legacy/offer";
