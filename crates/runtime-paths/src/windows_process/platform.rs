use std::path::PathBuf;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_INSUFFICIENT_BUFFER, HANDLE,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

/// Upper bound for an extended-length Windows path (`\\?\` prefixed), in UTF-16
/// code units. Buffer growth stops here.
const MAX_PATH_UTF16: usize = 32_768;

pub fn is_pid_alive(pid: u32) -> bool {
    unsafe {
        let Some(handle) = open_query_limited_process(pid) else {
            return false;
        };
        CloseHandle(handle);
        true
    }
}

pub fn process_executable_path(pid: u32) -> Option<PathBuf> {
    unsafe {
        let handle = open_query_limited_process(pid)?;
        let result = query_full_process_image_name(handle);
        CloseHandle(handle);
        result
    }
}

unsafe fn open_query_limited_process(pid: u32) -> Option<HANDLE> {
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        None
    } else {
        Some(handle)
    }
}

unsafe fn query_full_process_image_name(handle: HANDLE) -> Option<PathBuf> {
    let mut capacity = 1024usize;
    loop {
        let mut buffer = vec![0u16; capacity];
        let mut size = capacity as u32;
        let ok =
            QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, buffer.as_mut_ptr(), &mut size);
        if ok != 0 {
            let path = String::from_utf16_lossy(&buffer[..size as usize]);
            let path = path.trim();
            return if path.is_empty() {
                None
            } else {
                Some(PathBuf::from(path))
            };
        }

        if GetLastError() == ERROR_INSUFFICIENT_BUFFER && capacity < MAX_PATH_UTF16 {
            capacity = (capacity * 2).min(MAX_PATH_UTF16);
            continue;
        }
        return None;
    }
}
