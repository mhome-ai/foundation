//! Canonical public protocol exposed by the MeowLink App Facade.
//!
//! This crate deliberately contains no Core-to-Node runtime contracts. Public
//! callers use [`FacadeCall`] for every `/app/*` request; domain payloads live
//! under `input`, while invocation and presentation controls live under
//! `control`.

pub mod artifact;
pub mod call;
pub mod host_provision;
pub mod hub;
pub mod integration;
pub mod interaction_flow;
pub mod messaging;
pub mod plugin;
pub mod pod;
pub mod pod_call;
pub mod routing;
pub mod runtime;
pub mod topology;

pub use call::{ActionMode, FacadeCall, FacadeControl, ResponseDetail};

pub const PROTOCOL_VERSION: &str = "app-facade.v1";

pub mod system;

pub mod person;

pub mod user;

pub mod authorization;
pub mod delegation;
pub mod forwarding;

pub use conversation_api::execution::{FacadeProblem, FacadeResponse, FacadeStatus};

pub mod response;
