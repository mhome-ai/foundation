#![cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
// This outer file represents the future generated wrapper.
use mhome_script_sdk::prelude::*;
struct Input {
    samples: Vec<f64>,
    target_lux: f64,
    temperature: Temperature,
    delay: Duration,
    at: Timestamp,
    settings: Object,
}
mod user_script {
    use super::Input;
    use mhome_script_sdk::prelude::*;
    include!("lighting/source.rs");
}
fn invoke(value: Value) -> ScriptResult {
    let mut args = Arguments::new(value)?;
    let input = Input {
        samples: args.take("samples")?,
        target_lux: args.take("targetLux")?,
        temperature: args.take("temperature")?,
        delay: args.take("delay")?,
        at: args.take("at")?,
        settings: args.take("settings")?,
    };
    args.finish()?;
    user_script::run(input)
}
mhome_script_sdk::export_script!(invoke);
