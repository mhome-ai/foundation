//! Canonical Storage capability contracts. No database, filesystem or transport runtime.
pub mod backend;
pub mod error;
mod management;
mod model;
pub use error::*;
pub use management::*;
pub use model::*;
pub const PROTOCOL_VERSION: &str = "storage.v4";
