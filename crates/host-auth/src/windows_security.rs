//! Restrict named-pipe / token-file DACL to current user + SYSTEM + Administrators.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::ptr;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, FALSE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, SetFileSecurityW, TokenUser, DACL_SECURITY_INFORMATION,
    SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

pub struct PipeSecurity {
    descriptor: *mut core::ffi::c_void,
    attrs: SECURITY_ATTRIBUTES,
}

impl PipeSecurity {
    pub fn as_mut_ptr(&mut self) -> *mut core::ffi::c_void {
        std::ptr::addr_of_mut!(self.attrs) as *mut core::ffi::c_void
    }
}

impl Drop for PipeSecurity {
    fn drop(&mut self) {
        if !self.descriptor.is_null() {
            unsafe {
                LocalFree(self.descriptor);
            }
            self.descriptor = ptr::null_mut();
        }
    }
}

pub fn local_user_pipe_security() -> Result<PipeSecurity, String> {
    let (descriptor, attrs) = security_descriptor_for_local_user()?;
    Ok(PipeSecurity { descriptor, attrs })
}

pub fn apply_local_user_dacl(path: &Path) -> Result<(), String> {
    let (descriptor, _attrs) = security_descriptor_for_local_user()?;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let ok = unsafe { SetFileSecurityW(wide.as_ptr(), DACL_SECURITY_INFORMATION, descriptor) };
    unsafe {
        LocalFree(descriptor);
    }
    if ok == 0 {
        return Err(format!(
            "SetFileSecurityW failed for {}: {}",
            path.display(),
            std::io::Error::last_os_error()
        ));
    }
    Ok(())
}

fn security_descriptor_for_local_user(
) -> Result<(*mut core::ffi::c_void, SECURITY_ATTRIBUTES), String> {
    let user_sid = current_user_sid_string()?;
    // GA = Generic All for owner user, local system, and administrators.
    let sddl = format!("D:(A;;GA;;;{user_sid})(A;;GA;;;SY)(A;;GA;;;BA)");
    let wide: Vec<u16> = OsStr::new(&sddl)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut descriptor: *mut core::ffi::c_void = ptr::null_mut();
    let mut descriptor_size = 0_u32;
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            wide.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            &mut descriptor_size,
        )
    };
    if ok == 0 || descriptor.is_null() {
        return Err(format!(
            "ConvertStringSecurityDescriptorToSecurityDescriptorW failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: FALSE,
    };
    Ok((descriptor, attrs))
}

fn current_user_sid_string() -> Result<String, String> {
    unsafe {
        let mut token: HANDLE = INVALID_HANDLE_VALUE;
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return Err(format!(
                "OpenProcessToken failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut needed = 0_u32;
        let _ = GetTokenInformation(token, TokenUser, ptr::null_mut(), 0, &mut needed);
        if needed == 0 {
            let err = GetLastError();
            CloseHandle(token);
            return Err(format!("GetTokenInformation size failed: {err}"));
        }

        let mut buffer = vec![0_u8; needed as usize];
        if GetTokenInformation(
            token,
            TokenUser,
            buffer.as_mut_ptr() as *mut _,
            needed,
            &mut needed,
        ) == 0
        {
            let err = std::io::Error::last_os_error();
            CloseHandle(token);
            return Err(format!("GetTokenInformation failed: {err}"));
        }

        let token_user = &*(buffer.as_ptr() as *const TOKEN_USER);
        let mut sid_string: *mut u16 = ptr::null_mut();
        if ConvertSidToStringSidW(token_user.User.Sid, &mut sid_string) == 0 || sid_string.is_null()
        {
            let err = std::io::Error::last_os_error();
            CloseHandle(token);
            return Err(format!("ConvertSidToStringSidW failed: {err}"));
        }

        let sid = wide_nul_to_string(sid_string);
        LocalFree(sid_string as *mut core::ffi::c_void);
        CloseHandle(token);
        Ok(sid)
    }
}

unsafe fn wide_nul_to_string(ptr: *mut u16) -> String {
    let mut len = 0_usize;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len))
}
