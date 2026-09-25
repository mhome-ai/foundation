//! Transport-neutral Person contracts. No camera, storage or inference dependencies.
pub mod learning;
pub mod management;
pub mod matching;
pub mod model;
pub mod producer;
pub use learning::*;
pub use matching::*;
pub use model::*;
pub use producer::*;
pub const PROTOCOL_VERSION: &str = "person.v3";
pub const DESCRIBE_TARGET: &str = "/person/describe";
pub const OBSERVE_TARGET: &str = "/person/observe";
pub const MAX_SAMPLES_PER_BATCH: usize = 16;
pub const MAX_VECTOR_DIMENSIONS: usize = 4096;
pub const MAX_IMAGE_BYTES: usize = 512 * 1024;
pub const MATCH_POLICY: &str = "cosine-prototype.v2";

/// Live evidence is not a durable offline queue. No historical observation receipts are retained.
pub const MAX_EVIDENCE_AGE_MS: u64 = 5 * 60 * 1000;
