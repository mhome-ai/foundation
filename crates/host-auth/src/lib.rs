//! Shared Host authentication rules and optional pinned LAN TLS transport.
pub mod session;
pub mod signatures;
pub mod token_session;

#[cfg(feature = "tls")]
pub mod tls;

#[cfg(feature = "file-gateway")]
pub mod file_gateway;
