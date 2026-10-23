#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
// Fixed managed-build wrapper. No type inference or Rust source parsing here.
mod user_script {
    use mhome_script_sdk::prelude::*;
    include!("typed-lighting/source.rs");
}
mhome_script_sdk::export_typed_script!(user_script::Input, user_script::Output, user_script::run);
