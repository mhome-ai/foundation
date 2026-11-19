//! A humidity controller with explicit activity feedback and independent settings.
use super::*;
pub const HUMIDISTAT_V1: &str = "playground.humidistat.v1";
const CAPS: &[&str] = &[
    "state.humidistat",
    "state.humidistatSetting",
    "command.humidistatSetting",
    "data.humidistatAttributes",
    "state.humidity",
];
pub fn defaults() -> Map<String, Value> {
    object_map(json!({"online":true,"humidity":45,"humidityState":"idle","humidityMode":"auto","targetHumidity":50,"availableHumidityModes":["off","humidify","dehumidify","auto"],"humidityOperatingStates":["off","idle","humidifying","dehumidifying"],"minTargetHumidity":30,"maxTargetHumidity":80})).unwrap()
}
pub fn capabilities() -> Vec<String> {
    CAPS.iter().map(|s| s.to_string()).collect()
}
pub fn discovery(attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    ensure!(
        CAPS.contains(&cap),
        "Unsupported humidity controller capability"
    );
    object_map(
        json!({"supportedStates":["mode","targetHumidity"],"supportedSettings":["mode","targetHumidity"],"operatingStates":value_attr(attrs,"humidityOperatingStates")?,"availableModes":value_attr(attrs,"availableHumidityModes")?,"minTargetHumidity":number_attr(attrs,"minTargetHumidity")?,"maxTargetHumidity":number_attr(attrs,"maxTargetHumidity")?}),
    )
}
fn reading(attrs: &Map<String, Value>, key: &str, choices: &str) -> Result<Value> {
    let value = value_attr(attrs, key)?;
    if let Some(s) = value.as_str() {
        ensure_in_string_list(attrs, choices, s)?;
    } else {
        ensure!(value.is_null(), "Invalid humidity controller {key}");
    }
    Ok(value)
}
pub fn state(attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    match cap {
        "state.humidistat" => {
            object_map(json!({"state":reading(attrs,"humidityState","humidityOperatingStates")?}))
        }
        "state.humidistatSetting" => {
            let target = value_attr(attrs, "targetHumidity")?;
            ensure!(
                target.is_null()
                    || target
                        .as_f64()
                        .is_some_and(|n| n.is_finite() && (0.0..=100.0).contains(&n)),
                "Invalid target humidity feedback"
            );
            object_map(
                json!({"mode":reading(attrs,"humidityMode","availableHumidityModes")?,"targetHumidity":target}),
            )
        }
        "state.humidity" => object_map(json!({"humidity":number_attr(attrs,"humidity")?})),
        "data.humidistatAttributes" => discovery(attrs, cap),
        _ => Err(anyhow!("Unsupported humidity controller state")),
    }
}
pub fn observe(
    attrs: &mut Map<String, Value>,
    cap: &str,
    payload: &Map<String, Value>,
) -> Result<()> {
    let fields: &[(&str, &str)] = match cap {
        "state.humidistat" => &[("state", "humidityState")],
        "state.humidistatSetting" => &[
            ("mode", "humidityMode"),
            ("targetHumidity", "targetHumidity"),
        ],
        "state.humidity" => &[("humidity", "humidity")],
        _ => return Err(anyhow!("Unsupported humidity controller observation")),
    };
    ensure!(
        !payload.is_empty()
            && payload
                .keys()
                .all(|k| fields.iter().any(|(field, _)| field == k)),
        "Invalid humidity observation"
    );
    let mut next = attrs.clone();
    copy_optional_payload_fields(&mut next, payload, fields);
    state(&next, cap)?;
    *attrs = next;
    Ok(())
}
pub fn command(
    attrs: &mut Map<String, Value>,
    cap: &str,
    req: &Map<String, Value>,
) -> Result<CommandOutcome> {
    ensure!(
        cap == "command.humidistatSetting",
        "Unsupported humidity command"
    );
    ensure!(
        !req.is_empty()
            && req
                .keys()
                .all(|k| ["mode", "targetHumidity"].contains(&k.as_str())),
        "Specify humidity settings"
    );
    let mut next = attrs.clone();
    if let Some(mode) = req.get("mode") {
        let mode = mode
            .as_str()
            .ok_or_else(|| anyhow!("mode requires a string"))?;
        ensure_in_string_list(attrs, "availableHumidityModes", mode)?;
        next.insert("humidityMode".into(), json!(mode));
    }
    if let Some(target) = req.get("targetHumidity") {
        let n = target
            .as_f64()
            .filter(|n| n.is_finite())
            .ok_or_else(|| anyhow!("targetHumidity requires a number"))?;
        ensure!(
            n >= required_number(attrs, "minTargetHumidity")?
                && n <= required_number(attrs, "maxTargetHumidity")?,
            "Target humidity is outside the supported range"
        );
        next.insert("targetHumidity".into(), json!(n));
    }
    let payload = Value::Object(state(&next, "state.humidistatSetting")?);
    *attrs = next;
    Ok(CommandOutcome::with_event(
        "state.humidistatSetting",
        payload,
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn humidity_model_separates_readings_settings_activity_and_validates_before_mutating() {
        let mut a = defaults();
        let before = a.clone();
        for req in [
            json!({}),
            json!({"mode":"off","targetHumidity":81}),
            json!({"targetHumidity":"50"}),
            json!({"mode":"invalid"}),
        ] {
            assert!(command(
                &mut a,
                "command.humidistatSetting",
                req.as_object().unwrap()
            )
            .is_err());
            assert_eq!(a, before);
        }
        let event = command(
            &mut a,
            "command.humidistatSetting",
            json!({"mode":"humidify","targetHumidity":50.5})
                .as_object()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(event.events[0].capability, "state.humidistatSetting");
        assert_eq!(
            event.events[0].payload,
            json!({"mode":"humidify","targetHumidity":50.5})
        );
        assert_eq!(state(&a, "state.humidistat").unwrap()["state"], "idle");
        observe(
            &mut a,
            "state.humidistat",
            json!({"state":"humidifying"}).as_object().unwrap(),
        )
        .unwrap();
        assert_eq!(
            state(&a, "state.humidistat").unwrap()["state"],
            "humidifying"
        );
        observe(
            &mut a,
            "state.humidistatSetting",
            json!({"targetHumidity":null}).as_object().unwrap(),
        )
        .unwrap();
        assert!(state(&a, "state.humidistatSetting").unwrap()["targetHumidity"].is_null());
    }
}
