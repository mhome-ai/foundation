//! In-process operating-system permission calls.
//!
//! The caller is the process macOS will charge. This crate does not choose an
//! owner, persist install records, or expose HTTP.

pub use core_api::host::permissions::{PermissionEvidence, PermissionKey, PermissionState};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionObservation {
    pub permission: PermissionKey,
    pub state: PermissionState,
    pub evidence: PermissionEvidence,
    pub observed_at_ms: Option<i64>,
    pub error: Option<String>,
}

/// Passive read. This does not raise a system prompt. Local Network has no
/// passive API, so its state is the last request in this process.
pub fn status(permission: &PermissionKey) -> PermissionObservation {
    platform::status(permission)
}

/// Ask this process for the permission. Bluetooth opens System Settings because
/// reading its authorization does not prompt.
pub fn request(permission: &PermissionKey) -> Result<(), String> {
    platform::request(permission)
}

pub fn open_settings(permission: &PermissionKey) -> Result<(), String> {
    platform::open_settings(permission)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn observation(
    permission: PermissionKey,
    state: PermissionState,
    evidence: PermissionEvidence,
    observed_at_ms: Option<i64>,
    error: Option<String>,
) -> PermissionObservation {
    PermissionObservation {
        permission,
        state,
        evidence,
        observed_at_ms,
        error,
    }
}

#[cfg(not(target_os = "macos"))]
fn unsupported(permission: &PermissionKey, error: &str) -> PermissionObservation {
    observation(
        permission.clone(),
        PermissionState::Unsupported,
        PermissionEvidence::Unavailable,
        None,
        Some(error.to_string()),
    )
}

fn system_authorization(permission: PermissionKey, code: i32) -> PermissionObservation {
    let (state, error) = match code {
        0 => (PermissionState::NotDetermined, None),
        1 => (PermissionState::Restricted, None),
        2 => (PermissionState::Denied, None),
        3 => (PermissionState::Granted, None),
        4 => (
            PermissionState::Denied,
            Some("Reminders access is write-only.".into()),
        ),
        _ => (
            PermissionState::Unknown,
            Some("Permission could not be read.".into()),
        ),
    };
    let evidence = if error.is_none() || code == 4 {
        PermissionEvidence::System
    } else {
        PermissionEvidence::Unavailable
    };
    observation(permission, state, evidence, Some(now_ms()), error)
}

fn classify_dns_service(error: i32) -> Result<PermissionState, String> {
    match error {
        0 => Ok(PermissionState::Granted),
        -65570 => Ok(PermissionState::Denied),
        other => Err(format!("Local network check failed (DNS-SD {other})")),
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use std::ffi::{c_char, c_void, CString};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};

    #[derive(Clone)]
    struct NetworkObservation {
        state: PermissionState,
        at_ms: Option<i64>,
        error: Option<String>,
    }

    static NETWORK: Mutex<Option<NetworkObservation>> = Mutex::new(None);

    pub fn status(permission: &PermissionKey) -> PermissionObservation {
        match permission {
            PermissionKey::LocalNetwork {} => network_status(),
            PermissionKey::Bluetooth {} => system_authorization(permission.clone(), unsafe {
                os_permissions_bluetooth_authorization()
            }),
            PermissionKey::Microphone {} => system_authorization(permission.clone(), unsafe {
                os_permissions_microphone_authorization()
            }),
            PermissionKey::Reminders {} => system_authorization(permission.clone(), unsafe {
                os_permissions_reminders_authorization()
            }),
            PermissionKey::Automation { target_bundle_id } => {
                automation_status(target_bundle_id, false)
            }
        }
    }

    pub fn request(permission: &PermissionKey) -> Result<(), String> {
        match permission {
            PermissionKey::LocalNetwork {} => {
                let state = probe_local_network()?;
                let mut slot = NETWORK
                    .lock()
                    .map_err(|_| "Permission state is unavailable".to_string())?;
                *slot = Some(NetworkObservation {
                    state,
                    at_ms: Some(now_ms()),
                    error: None,
                });
                Ok(())
            }
            PermissionKey::Bluetooth {} => open_settings(permission),
            PermissionKey::Microphone {} => {
                let code = unsafe { os_permissions_microphone_request() };
                if code < 0 {
                    Err("Microphone permission request timed out".into())
                } else {
                    Ok(())
                }
            }
            PermissionKey::Reminders {} => {
                let code = unsafe { os_permissions_reminders_request() };
                if code < 0 {
                    Err("Reminders permission request timed out".into())
                } else {
                    Ok(())
                }
            }
            PermissionKey::Automation { target_bundle_id } => {
                let observed = automation_status(target_bundle_id, true);
                match observed.error {
                    Some(error) => Err(error),
                    None => Ok(()),
                }
            }
        }
    }

    pub fn open_settings(permission: &PermissionKey) -> Result<(), String> {
        let pane = match permission {
            PermissionKey::LocalNetwork {} => "Privacy_LocalNetwork",
            PermissionKey::Bluetooth {} => "Privacy_Bluetooth",
            PermissionKey::Microphone {} => "Privacy_Microphone",
            PermissionKey::Reminders {} => "Privacy_Reminders",
            PermissionKey::Automation { .. } => "Privacy_Automation",
        };
        let status = std::process::Command::new("/usr/bin/open")
            .arg(format!(
                "x-apple.systempreferences:com.apple.preference.security?{pane}"
            ))
            .status()
            .map_err(|error| error.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("Could not open System Settings".into())
        }
    }

    fn network_status() -> PermissionObservation {
        let slot = NETWORK.lock().ok().and_then(|slot| slot.clone());
        match slot {
            Some(item) => observation(
                PermissionKey::LocalNetwork {},
                item.state,
                PermissionEvidence::Probe,
                item.at_ms,
                item.error,
            ),
            None => observation(
                PermissionKey::LocalNetwork {},
                PermissionState::Unknown,
                PermissionEvidence::Unavailable,
                None,
                None,
            ),
        }
    }

    fn automation_status(bundle_id: &str, ask: bool) -> PermissionObservation {
        let permission = PermissionKey::Automation {
            target_bundle_id: bundle_id.to_string(),
        };
        let c_bundle = match CString::new(bundle_id) {
            Ok(value) => value,
            Err(_) => {
                return observation(
                    permission,
                    PermissionState::Unknown,
                    PermissionEvidence::Unavailable,
                    None,
                    Some("Automation target is not valid".into()),
                );
            }
        };
        let code = unsafe { os_permissions_automation(c_bundle.as_ptr(), if ask { 1 } else { 0 }) };
        let (state, evidence, error) = match code {
            0 => (PermissionState::Granted, PermissionEvidence::System, None),
            -1743 => (PermissionState::Denied, PermissionEvidence::System, None),
            -1744 => (
                PermissionState::NotDetermined,
                PermissionEvidence::System,
                None,
            ),
            -600 => (
                PermissionState::Unknown,
                PermissionEvidence::Unavailable,
                Some("The application to automate is not running.".into()),
            ),
            other => (
                PermissionState::Unknown,
                PermissionEvidence::Unavailable,
                Some(format!("Automation permission could not be read ({other})")),
            ),
        };
        observation(
            permission,
            state,
            evidence,
            if evidence == PermissionEvidence::System {
                Some(now_ms())
            } else {
                None
            },
            error,
        )
    }

    fn probe_local_network() -> Result<PermissionState, String> {
        let name = CString::new(format!(
            "meow-permission-{}-{}",
            std::process::id(),
            now_ms()
        ))
        .map_err(|error| error.to_string())?;
        let kind = CString::new("_meow-permission._tcp").unwrap();
        let domain = CString::new("local.").unwrap();
        let mut result: Option<i32> = None;
        let mut raw = std::ptr::null_mut();
        let error = unsafe {
            DNSServiceRegister(
                &mut raw,
                0,
                0,
                name.as_ptr(),
                kind.as_ptr(),
                domain.as_ptr(),
                std::ptr::null(),
                9u16.to_be(),
                0,
                std::ptr::null(),
                registered,
                &mut result as *mut _ as *mut c_void,
            )
        };
        if error != 0 {
            return classify_dns_service(error);
        }
        if raw.is_null() {
            return Err("Local network check returned no registration".into());
        }
        let registration = Registration(raw);
        let fd = unsafe { DNSServiceRefSockFD(registration.0) };
        if fd < 0 {
            return Err("Local network check returned no socket".into());
        }
        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            let mut poll = libc::pollfd {
                fd,
                events: libc::POLLIN,
                revents: 0,
            };
            let ready = unsafe { libc::poll(&mut poll, 1, 100) };
            if ready < 0 {
                let error = std::io::Error::last_os_error();
                if error.kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(format!("Local network check failed: {error}"));
            }
            if ready > 0 {
                let error = unsafe { DNSServiceProcessResult(registration.0) };
                if error != 0 {
                    return classify_dns_service(error);
                }
                if let Some(error) = result {
                    return classify_dns_service(error);
                }
            }
        }
        Ok(PermissionState::Unknown)
    }

    struct Registration(*mut c_void);
    impl Drop for Registration {
        fn drop(&mut self) {
            unsafe { DNSServiceRefDeallocate(self.0) }
        }
    }

    extern "C" fn registered(
        _: *mut c_void,
        _: u32,
        error: i32,
        _: *const c_char,
        _: *const c_char,
        _: *const c_char,
        context: *mut c_void,
    ) {
        unsafe {
            *(context as *mut Option<i32>) = Some(error);
        }
    }

    extern "C" {
        fn os_permissions_bluetooth_authorization() -> i32;
        fn os_permissions_microphone_authorization() -> i32;
        fn os_permissions_microphone_request() -> i32;
        fn os_permissions_reminders_authorization() -> i32;
        fn os_permissions_reminders_request() -> i32;
        fn os_permissions_automation(bundle_id: *const c_char, ask: i32) -> i32;
        fn DNSServiceRegister(
            sd: *mut *mut c_void,
            flags: u32,
            index: u32,
            name: *const c_char,
            kind: *const c_char,
            domain: *const c_char,
            host: *const c_char,
            port: u16,
            txt_len: u16,
            txt: *const c_void,
            callback: extern "C" fn(
                *mut c_void,
                u32,
                i32,
                *const c_char,
                *const c_char,
                *const c_char,
                *mut c_void,
            ),
            context: *mut c_void,
        ) -> i32;
        fn DNSServiceRefSockFD(sd: *mut c_void) -> i32;
        fn DNSServiceProcessResult(sd: *mut c_void) -> i32;
        fn DNSServiceRefDeallocate(sd: *mut c_void);
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub fn status(permission: &PermissionKey) -> PermissionObservation {
        unsupported(
            permission,
            "This operating system has no system permission to read.",
        )
    }

    pub fn request(_permission: &PermissionKey) -> Result<(), String> {
        Err("Permission requests are only available on macOS".into())
    }

    pub fn open_settings(_permission: &PermissionKey) -> Result<(), String> {
        Err("System Settings are only available on macOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dns_service_denial_is_distinct_from_other_failures() {
        assert_eq!(classify_dns_service(0).unwrap(), PermissionState::Granted);
        assert_eq!(
            classify_dns_service(-65570).unwrap(),
            PermissionState::Denied
        );
        assert!(classify_dns_service(-65563).is_err());
    }

    #[test]
    fn authorization_codes_map_without_calling_the_system() {
        let granted = system_authorization(PermissionKey::Microphone {}, 3);
        assert_eq!(granted.state, PermissionState::Granted);
        assert_eq!(granted.evidence, PermissionEvidence::System);
        let write_only = system_authorization(PermissionKey::Reminders {}, 4);
        assert_eq!(write_only.state, PermissionState::Denied);
        assert!(write_only.error.is_some());
        let unreadable = system_authorization(PermissionKey::Bluetooth {}, -1);
        assert_eq!(unreadable.state, PermissionState::Unknown);
        assert_eq!(unreadable.evidence, PermissionEvidence::Unavailable);
    }
}
