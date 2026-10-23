//! Guest ownership: host owns input allocation until invoke returns; guest owns
//! output until host copies it and calls script_dealloc. A trap drops the Store.
use crate::{codec, ScriptContract, ScriptError, ScriptInput, ScriptOutput, ScriptResult, Value};

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
    return_bytes(dispatch(bytes, run))
}

#[cfg(target_arch = "wasm32")]
pub fn return_bytes(bytes: Vec<u8>) -> u64 {
    let out = bytes.into_boxed_slice();
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

/// Converts arguments using the Rust input type and always emits an output object.
pub fn run_typed<I: ScriptInput, O: ScriptOutput>(
    value: Value,
    run: fn(I) -> Result<O, ScriptError>,
) -> ScriptResult {
    run(I::from_value(value)?).map(ScriptOutput::into_value)
}

/// Fixed wrapper for managed builds. The source declares both types and `run`.
/// No business function is called to obtain the contract.
///
/// ```compile_fail
/// use mhome_script_sdk::prelude::*;
/// #[derive(ScriptInput)]
/// struct Input { x: f64 }
/// #[derive(ScriptOutput)]
/// struct Output { y: f64 }
/// fn run(_: Input) -> f64 { 0.0 }
/// mhome_script_sdk::export_typed_script!(Input, Output, run);
/// ```
#[macro_export]
macro_rules! export_typed_script {
    ($input:ty, $output:ty, $run:path) => {
        // Check the signature on native builds too, not only on wasm32.
        const _: fn($input) -> ::std::result::Result<$output, $crate::ScriptError> = $run;
        #[cfg(target_arch = "wasm32")]
        fn __script_invoke_typed(value: $crate::Value) -> $crate::ScriptResult {
            $crate::guest::run_typed::<$input, $output>(value, $run)
        }
        $crate::export_script!(__script_invoke_typed);
        #[cfg(target_arch = "wasm32")]
        #[no_mangle]
        pub extern "C" fn script_contract() -> u64 {
            match $crate::guest::contract_bytes::<$input, $output>() {
                Ok(bytes) => $crate::guest::return_bytes(bytes),
                Err(_) => 0,
            }
        }
    };
}

pub fn contract_bytes<I: ScriptInput, O: ScriptOutput>() -> Result<Vec<u8>, ScriptError> {
    ScriptContract::for_script::<I, O>().encode(codec::Limits::default())
}
