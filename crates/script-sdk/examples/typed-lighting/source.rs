#[derive(ScriptInput)]
pub struct Input {
    pub samples: Vec<f64>,
    pub target_lux: f64,
    pub temperature: Temperature,
    pub delay: Duration,
    pub at: Timestamp,
    pub settings: Object,
    pub image: Image,
    pub items: Vec<Object>,
    pub data: Value,
    pub matrix: Vec<Vec<f64>>,
}

#[derive(ScriptOutput)]
pub struct Output {
    pub brightness: f64,
    pub levels: Vec<f64>,
    pub temperature: Temperature,
    pub delay: Duration,
    pub at: Timestamp,
    pub settings: Object,
    pub image: Image,
    pub items: Vec<Object>,
    pub data: Value,
    pub matrix: Vec<Vec<f64>>,
}

pub fn run(input: Input) -> Result<Output, ScriptError> {
    if input.samples.is_empty() || input.target_lux <= 0.0 {
        return Err(ScriptError::new(
            "INVALID_INPUT",
            "samples and positive target required",
        ));
    }
    let average = input.samples.iter().sum::<f64>() / input.samples.len() as f64;
    Ok(Output {
        brightness: (100.0 * (1.0 - average / input.target_lux)).clamp(0.0, 100.0),
        levels: input.samples,
        temperature: input.temperature,
        delay: input.delay,
        at: input.at,
        settings: input.settings,
        image: input.image,
        items: input.items,
        data: input.data,
        matrix: input.matrix,
    })
}
