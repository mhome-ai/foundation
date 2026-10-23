//! Guest ownership: host owns input allocation until invoke returns; guest owns
//! output until host copies it and calls script_dealloc. A trap drops the Store.
use crate::{codec, ScriptResult, Value};

pub fn dispatch(bytes: &[u8], run: fn(Value) -> ScriptResult) -> Vec<u8> {
    let limits = codec::Limits::default();
    let result = codec::decode(bytes, limits).and_then(run);
    codec::encode_result(&result, limits).unwrap_or_else(|_| {
        codec::encode_result(
            &Err(crate::ScriptError::new(
                "INVALID_OUTPUT",
                "output exceeds codec limits",
            )),
            limits,
        )
        .unwrap()
    })
}

#[cfg(target_arch = "wasm32")]
pub fn allocate(len: u32) -> u32 {
    if len == 0 || len as usize > codec::Limits::default().bytes {
        return 0;
    }
    let bytes = vec![0u8; len as usize].into_boxed_slice();
    Box::into_raw(bytes) as *mut u8 as u32
}
/// # Safety
/// `ptr,len` must identify a live allocation returned by allocate or invoke.
#[cfg(target_arch = "wasm32")]
pub unsafe fn deallocate(ptr: u32, len: u32) {
    if ptr != 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut u8,
            len as usize,
        )));
    }
}
/// # Safety
/// Input points to a live allocation of at least len bytes. It is only borrowed.
#[cfg(target_arch = "wasm32")]
pub unsafe fn invoke(ptr: u32, len: u32, run: fn(Value) -> ScriptResult) -> u64 {
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len as usize);
    let out = dispatch(bytes, run).into_boxed_slice();
    let len = out.len() as u32;
    let ptr = Box::into_raw(out) as *mut u8 as u32;
    ((ptr as u64) << 32) | len as u64
}
/// Emitted by the build wrapper, never required in the user's source file.
#[macro_export]
macro_rules! export_script {
    ($run:path) => {
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn script_abi_version() -> i32 {
            $crate::ABI_VERSION
        }
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn script_alloc(len: u32) -> u32 {
            $crate::guest::allocate(len)
        }
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub unsafe extern "C" fn script_dealloc(ptr: u32, len: u32) {
            $crate::guest::deallocate(ptr, len)
        }
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub unsafe extern "C" fn script_invoke(ptr: u32, len: u32) -> u64 {
            $crate::guest::invoke(ptr, len, $run)
        }
    };
}
