//! In-process operating-system permission calls.
//!
//! These observations describe the calling process's execution context. macOS
//! may attribute that access to a responsible application, including Host. This
//! crate does not discover a grant owner or query another process's permission.

pub use core_api::host::permissions::{
    PermissionEvidence, PermissionKey, PermissionObservation, PermissionState,
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Passive read. This does not raise a system prompt. Local Network has no
/// passive API, so its state is the latest observation from an explicitly started
/// network monitor in this process. Reading status never starts that monitor.
pub fn status(permission: &PermissionKey) -> PermissionObservation {
    platform::status(permission)
}

/// Explicitly request authorization in this process's execution context.
/// Returns after starting the request, without waiting for the user. Repeated
/// requests for the same permission reuse the pending request. Poll status for
/// approval; successful submission is not a grant. Opening Settings is separate.
pub fn request(permission: &PermissionKey) -> Result<(), String> {
    platform::request(permission)
}

pub fn open_settings(permission: &PermissionKey) -> Result<(), String> {
    platform::open_settings(permission)
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
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
        process_id: std::process::id(),
        state,
        evidence,
        observed_at_ms,
        error,
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn unsupported(permission: &PermissionKey, error: &str) -> PermissionObservation {
    observation(
        permission.clone(),
        PermissionState::Unsupported,
        PermissionEvidence::Unavailable,
        None,
        Some(error.to_string()),
    )
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
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

#[cfg(any(target_os = "linux", test))]
const LINUX_BLUETOOTH_GROUP: &str = "bluetooth";

/// Guidance for a Bluetooth request that BlueZ refused with `AccessDenied` or
/// `NotAuthorized`. Only call it after such a refusal: BlueZ policy decides
/// access, and group membership alone does not predict it. `None` when the
/// group cannot explain the refusal or off Linux.
pub fn bluetooth_access_denied_hint() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let read = |path: &str| std::fs::read_to_string(path).unwrap_or_default();
        linux_bluetooth_hint(
            &read("/proc/self/status"),
            &read("/etc/group"),
            &read("/etc/passwd"),
        )
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// `status` is `/proc/self/status`; `groups` and `passwd` are `/etc/group` and
/// `/etc/passwd`. A systemd user manager keeps the groups it started with, so a
/// service under it gains a new group only after that manager restarts.
#[cfg(any(target_os = "linux", test))]
fn linux_bluetooth_hint(status: &str, groups: &str, passwd: &str) -> Option<String> {
    let ids = |key: &str| -> Vec<u32> {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .map(|rest| {
                rest.split_whitespace()
                    .filter_map(|id| id.parse().ok())
                    .collect()
            })
            .unwrap_or_default()
    };
    let uid = *ids("Uid:").get(1)?;
    if uid == 0 {
        return None;
    }
    let (gid, members) = groups.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        (fields.len() >= 4 && fields[0] == LINUX_BLUETOOTH_GROUP)
            .then(|| fields[2].parse::<u32>().ok().map(|gid| (gid, fields[3])))
            .flatten()
    })?;
    if ids("Gid:").get(1) == Some(&gid) || ids("Groups:").contains(&gid) {
        return None;
    }
    let user = passwd.lines().find_map(|line| {
        let fields: Vec<&str> = line.split(':').collect();
        (fields.len() >= 3 && fields[2].parse::<u32>().ok() == Some(uid)).then_some(fields[0])
    });
    let restart = format!(
        "restart the user service manager (sudo systemctl restart user@{uid}.service) or reboot; signing out is not enough while another session or lingering keeps it running."
    );
    let listed = user.is_some_and(|user| members.split(',').any(|member| member == user));
    Some(if listed {
        format!("This user is in the {LINUX_BLUETOOTH_GROUP} group, but this process started before it was added. To apply it, {restart}")
    } else {
        let user = user.map_or_else(|| "$USER".to_string(), str::to_string);
        format!("Add this user to the {LINUX_BLUETOOTH_GROUP} group (sudo usermod -aG {LINUX_BLUETOOTH_GROUP} {user}), then {restart}")
    })
}

#[cfg(any(target_os = "linux", test))]
fn linux_status(permission: &PermissionKey) -> PermissionObservation {
    match permission {
        PermissionKey::LocalNetwork {}
        | PermissionKey::Bluetooth {}
        | PermissionKey::Microphone {} => observation(
            permission.clone(),
            PermissionState::NotRequired,
            PermissionEvidence::Platform,
            Some(now_ms()),
            None,
        ),
        PermissionKey::Reminders {} | PermissionKey::Automation { .. } => observation(
            permission.clone(),
            PermissionState::Unsupported,
            PermissionEvidence::Unavailable,
            None,
            Some("This permission is specific to macOS.".into()),
        ),
    }
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn classify_dns_service(error: i32) -> Result<PermissionState, String> {
    match error {
        0 => Ok(PermissionState::Granted),
        -65570 => Ok(PermissionState::Denied),
        other => Err(format!("Local network check failed (DNS-SD {other})")),
    }
}

#[cfg(any(target_os = "macos", test))]
mod requests;

#[cfg(target_os = "macos")]
mod platform {
    use super::requests::Requests;
    use super::*;
    use std::ffi::{c_char, c_void, CString};
    use std::sync::{LazyLock, Mutex};

    static REQUESTS: LazyLock<Requests> = LazyLock::new(Requests::default);
    static NETWORK: Mutex<Option<PermissionObservation>> = Mutex::new(None);

    pub fn status(permission: &PermissionKey) -> PermissionObservation {
        let mut result = match permission {
            PermissionKey::LocalNetwork {} => NETWORK
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
                .unwrap_or_else(|| {
                    observation(
                        permission.clone(),
                        PermissionState::Unknown,
                        PermissionEvidence::Unavailable,
                        None,
                        None,
                    )
                }),
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
        };
        if matches!(
            result.state,
            PermissionState::Unknown | PermissionState::NotDetermined
        ) {
            if let Some(error) = REQUESTS.error(permission) {
                result.error = Some(error);
            }
        }
        result
    }

    pub fn request(permission: &PermissionKey) -> Result<(), String> {
        if let PermissionKey::Automation { target_bundle_id } = permission {
            if target_bundle_id.trim().is_empty()
                || CString::new(target_bundle_id.as_str()).is_err()
            {
                return Err("Automation target is not valid".into());
            }
        }
        if !REQUESTS.begin(permission) {
            return Ok(());
        }
        match permission {
            PermissionKey::LocalNetwork {} => {
                // The native monitor remains alive after an observation so Settings
                // changes can update it. Only explicit requests start/restart it.
                *NETWORK.lock().unwrap_or_else(|e| e.into_inner()) = None;
                unsafe {
                    os_permissions_network_request(network_changed);
                }
            }
            PermissionKey::Automation { target_bundle_id } => {
                let key = permission.clone();
                let target = target_bundle_id.clone();
                // Apple Events has a synchronous API. At most one worker per target
                // remains blocked while the OS waits; caller timeouts don't release it.
                if let Err(error) = std::thread::Builder::new()
                    .name("permission-automation".into())
                    .spawn(move || {
                        let result = automation_status(&target, true);
                        REQUESTS.finish(&key, result.error);
                    })
                {
                    let error = format!("Could not start authorization: {error}");
                    REQUESTS.finish(permission, Some(error.clone()));
                    return Err(error);
                }
            }
            _ => {
                let context = Box::into_raw(Box::new(permission.clone())).cast::<c_void>();
                unsafe {
                    match permission {
                        PermissionKey::Bluetooth {} => {
                            os_permissions_bluetooth_request(native_completed, context)
                        }
                        PermissionKey::Microphone {} => {
                            os_permissions_microphone_request(native_completed, context)
                        }
                        PermissionKey::Reminders {} => {
                            os_permissions_reminders_request(native_completed, context)
                        }
                        _ => unreachable!(),
                    }
                }
            }
        }
        Ok(())
    }

    extern "C" fn native_completed(context: *mut c_void, code: i32) {
        // Every native request owns this context and completes exactly once.
        let key = unsafe { Box::from_raw(context.cast::<PermissionKey>()) };
        REQUESTS.finish(
            &key,
            (code < 0).then(|| "The system could not complete the permission request.".into()),
        );
    }

    extern "C" fn network_changed(code: i32) {
        let (state, error) = match classify_dns_service(code) {
            Ok(state) => (state, None),
            Err(error) => (PermissionState::Unknown, Some(error)),
        };
        *NETWORK.lock().unwrap_or_else(|e| e.into_inner()) = Some(observation(
            PermissionKey::LocalNetwork {},
            state,
            PermissionEvidence::Probe,
            Some(now_ms()),
            error.clone(),
        ));
        REQUESTS.finish(&PermissionKey::LocalNetwork {}, error);
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
                )
            }
        };
        let code = unsafe { os_permissions_automation(c_bundle.as_ptr(), i32::from(ask)) };
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
            (evidence == PermissionEvidence::System).then(now_ms),
            error,
        )
    }

    type Completion = extern "C" fn(*mut c_void, i32);
    extern "C" {
        fn os_permissions_bluetooth_authorization() -> i32;
        fn os_permissions_bluetooth_request(done: Completion, context: *mut c_void);
        fn os_permissions_microphone_authorization() -> i32;
        fn os_permissions_microphone_request(done: Completion, context: *mut c_void);
        fn os_permissions_reminders_authorization() -> i32;
        fn os_permissions_reminders_request(done: Completion, context: *mut c_void);
        fn os_permissions_automation(bundle_id: *const c_char, ask: i32) -> i32;
        fn os_permissions_network_request(changed: extern "C" fn(i32));
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::*;

    pub fn status(permission: &PermissionKey) -> PermissionObservation {
        #[cfg(target_os = "linux")]
        {
            linux_status(permission)
        }
        #[cfg(target_os = "windows")]
        {
            match permission {
                PermissionKey::Bluetooth {} => windows_bluetooth_status(),
                _ => unsupported(
                    permission,
                    "Permission observation is not implemented for this platform.",
                ),
            }
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            unsupported(
                permission,
                "Permission observation is not implemented for this platform.",
            )
        }
    }

    /// Desktop apps need no Bluetooth consent on Windows; without a radio the
    /// permission cannot be used at all.
    #[cfg(target_os = "windows")]
    fn windows_bluetooth_status() -> PermissionObservation {
        use windows_sys::Win32::Devices::Bluetooth::{
            BluetoothFindFirstRadio, BluetoothFindRadioClose, BLUETOOTH_FIND_RADIO_PARAMS,
        };
        use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};

        let params = BLUETOOTH_FIND_RADIO_PARAMS {
            dwSize: std::mem::size_of::<BLUETOOTH_FIND_RADIO_PARAMS>() as u32,
        };
        let mut radio: HANDLE = std::ptr::null_mut();
        let find = unsafe { BluetoothFindFirstRadio(&params, &mut radio) };
        if find.is_null() {
            return unsupported(
                &PermissionKey::Bluetooth {},
                "No Bluetooth adapter was found.",
            );
        }
        unsafe {
            CloseHandle(radio);
            BluetoothFindRadioClose(find);
        }
        observation(
            PermissionKey::Bluetooth {},
            PermissionState::NotRequired,
            PermissionEvidence::Platform,
            Some(now_ms()),
            None,
        )
    }

    pub fn request(_permission: &PermissionKey) -> Result<(), String> {
        Err("This platform has no supported application-consent request.".into())
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
        assert_eq!(granted.process_id, std::process::id());
    }

    #[test]
    fn linux_bluetooth_hints_follow_group_membership() {
        let groups = "root:x:0:\nbluetooth:x:112:alice\n";
        let passwd = "root:x:0:0::/root:/bin/sh\nmeow:x:1000:1000::/home/meow:/bin/sh\nalice:x:1001:1001::/home/alice:/bin/sh\n";
        let user = |uid: u32, gid: u32, extra: &str| {
            format!("Name:\tmeow\nUid:\t{uid}\t{uid}\t{uid}\t{uid}\nGid:\t{gid}\t{gid}\t{gid}\t{gid}\nGroups:\t{extra}\n")
        };
        let missing = linux_bluetooth_hint(&user(1000, 1000, "27 1000"), groups, passwd).unwrap();
        assert!(missing.contains("sudo usermod -aG bluetooth meow"));
        assert!(missing.contains("user@1000.service"));
        let stale = linux_bluetooth_hint(&user(1001, 1001, "1001"), groups, passwd).unwrap();
        assert!(stale.contains("started before it was added"));
        assert!(stale.contains("user@1001.service"));
        assert!(linux_bluetooth_hint(&user(1000, 1000, "27 112 1000"), groups, passwd).is_none());
        assert!(linux_bluetooth_hint(&user(1000, 112, ""), groups, passwd).is_none());
        assert!(linux_bluetooth_hint(&user(1000, 1000, ""), "root:x:0:\n", passwd).is_none());
        assert!(linux_bluetooth_hint(&user(0, 0, "0"), groups, passwd).is_none());
        let unnamed = linux_bluetooth_hint(&user(1500, 1500, ""), groups, passwd).unwrap();
        assert!(unnamed.contains("sudo usermod -aG bluetooth $USER"));
    }

    #[test]
    fn linux_native_permissions_require_no_application_consent() {
        for key in [
            PermissionKey::LocalNetwork {},
            PermissionKey::Bluetooth {},
            PermissionKey::Microphone {},
        ] {
            let observed = linux_status(&key);
            assert_eq!(observed.state, PermissionState::NotRequired);
            assert_eq!(observed.evidence, PermissionEvidence::Platform);
            assert_eq!(observed.process_id, std::process::id());
            assert!(observed.error.is_none());
        }
        assert_eq!(
            linux_status(&PermissionKey::Reminders {}).state,
            PermissionState::Unsupported
        );
        assert_eq!(
            linux_status(&PermissionKey::Automation {
                target_bundle_id: "com.apple.Music".into()
            })
            .state,
            PermissionState::Unsupported
        );
    }
}
