use std::path::PathBuf;
use std::process::{Command, Stdio};

pub fn is_pid_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn process_executable_path(pid: u32) -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_link(format!("/proc/{}/exe", pid)).ok()
    }
    #[cfg(target_os = "macos")]
    {
        // proc_pidpath returns the full image path, including spaces in .app
        // bundles (e.g. "/Applications/Meow App.app/.../meowclient"). Do not
        // parse `ps -o args=` — whitespace splitting truncates those paths.
        let mut buf = [0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        let len = unsafe {
            libc::proc_pidpath(
                pid as libc::c_int,
                buf.as_mut_ptr() as *mut libc::c_void,
                buf.len() as u32,
            )
        };
        if len <= 0 {
            return None;
        }
        let path = std::ffi::CStr::from_bytes_until_nul(&buf[..]).ok()?;
        let path = path.to_str().ok()?;
        if path.is_empty() {
            return None;
        }
        Some(PathBuf::from(path))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = pid;
        None
    }
}

/// Best-effort peer process id for a connected Unix domain socket.
pub fn unix_peer_pid(fd: std::os::unix::io::RawFd) -> Option<u32> {
    #[cfg(target_os = "macos")]
    {
        // LOCAL_PEERPID from sys/un.h / sys/socket.h
        const LOCAL_PEERPID: libc::c_int = 0x002;
        let mut pid: libc::pid_t = 0;
        let mut len = std::mem::size_of::<libc::pid_t>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                LOCAL_PEERPID,
                &mut pid as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };
        if rc == 0 && pid > 0 {
            Some(pid as u32)
        } else {
            None
        }
    }
    #[cfg(target_os = "linux")]
    {
        let mut cred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let rc = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut _ as *mut libc::c_void,
                &mut len,
            )
        };
        if rc == 0 && cred.pid > 0 {
            Some(cred.pid as u32)
        } else {
            None
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = fd;
        None
    }
}

/// Remove a Unix domain socket path only when nothing is accepting connections.
///
/// Prefer bind-first + retry in the daemon server over calling this from a
/// parent before spawn: connect→unlink still has a TOCTOU window if another
/// process binds between those steps.
///
/// Returns `Ok(true)` if a stale socket was removed, `Ok(false)` if the path was
/// absent or still live (left untouched).
pub fn clear_stale_unix_socket(socket_path: &std::path::Path) -> std::io::Result<bool> {
    if !socket_path.exists() {
        return Ok(false);
    }
    // A successful connect means a live listener still owns this path.
    if std::os::unix::net::UnixStream::connect(socket_path).is_ok() {
        return Ok(false);
    }
    match std::fs::remove_file(socket_path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}
