//! Shared Host authentication rules and optional pinned LAN TLS transport.
pub mod access;
pub mod policy;
pub mod signatures;

#[cfg(feature = "tls")]
pub mod tls;

#[cfg(feature = "file-gateway")]
pub mod file_gateway;

#[cfg(feature = "authority")]
pub mod authority;
#[cfg(feature = "local-state")]
pub mod device_binding;
#[cfg(feature = "local-state")]
pub mod secure_file;
#[cfg(feature = "local-state")]
pub mod service_control;
#[cfg(all(feature = "local-state", windows))]
pub mod windows_security;
