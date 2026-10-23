pub fn run(input: Input) -> ScriptResult {
    if input.samples.is_empty() || input.target_lux <= 0.0 {
        return Err(ScriptError::new("INVALID_INPUT", "samples and positive target required"));
    }
    let average = input.samples.iter().sum::<f64>() / input.samples.len() as f64;
    Ok(object! {
        "brightness" => (100.0 * (1.0 - average / input.target_lux)).clamp(0.0, 100.0),
        "temperature" => input.temperature,
        "delay" => input.delay,
        "at" => input.at,
        "settings" => input.settings,
    }.into())
}
