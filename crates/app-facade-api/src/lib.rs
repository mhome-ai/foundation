//! Canonical public protocol exposed by the MeowLink App Facade.
//!
//! This crate deliberately contains no Core-to-Node runtime contracts. Public
//! callers use [`FacadeCall`] for every `/app/*` request; domain payloads live
//! under `input`, while invocation and presentation controls live under
//! `control`.

pub mod call;
pub mod interaction_flow;
pub mod messaging;
pub mod plugin;
pub mod registry;
pub mod runtime;

pub use call::{ActionMode, FacadeCall, FacadeControl, ResponseFormat};

pub const PROTOCOL_VERSION: &str = "app-facade.v1";
