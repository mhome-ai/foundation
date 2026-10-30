//! Core owns one Catalog. Storage plugins expose only Disk and scoped byte grants.
pub mod catalog;
pub mod disk;
pub mod model;
pub use catalog::Catalog;
pub use disk::Disk;
pub use model::*;

pub mod temporary;
