#[cfg(windows)]
mod platform;

#[cfg(windows)]
pub use platform::{is_pid_alive, process_executable_path};
