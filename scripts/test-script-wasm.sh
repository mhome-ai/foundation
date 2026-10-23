#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build -p mhome-script-sdk --example lighting --target wasm32-unknown-unknown --release --locked
SCRIPT_SDK_WASM="${CARGO_TARGET_DIR:-$PWD/target}/wasm32-unknown-unknown/release/examples/lighting.wasm"
export SCRIPT_SDK_WASM
cargo test -p mhome-script-sdk -p mhome-wasm-runtime --locked
cargo test -p mhome-wasm-runtime --locked rust_sdk_fixture -- --ignored
