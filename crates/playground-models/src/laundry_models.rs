//! Deterministic washer, dryer and combined-appliance tasks.
use super::*;
pub const WASHER_V1: &str = "playground.laundry.washer.v1";
pub const DRYER_V1: &str = "playground.laundry.dryer.v1";
pub const WASHER_DRYER_V1: &str = "playground.laundry.washer-dryer.v1";
const CAPS: &[&str] = &[
    "state.laundry",
    "state.laundrySetting",
    "command.laundryControl",
    "command.laundrySetting",
    "data.laundryAttributes",
];
const STATES: &[&str] = &["idle", "running", "paused", "stopped", "completed", "error"];
pub fn supported(id: &str) -> bool {
    matches!(id, WASHER_V1 | DRYER_V1 | WASHER_DRYER_V1)
}
fn fields(id: &str) -> Vec<(&'static str, &'static str)> {
    let mut f = vec![("program", "availablePrograms")];
    if id != DRYER_V1 {
        f.extend([
            ("spinSpeed", "availableSpinSpeeds"),
            ("rinse", "availableRinses"),
        ]);
    }
    if id != WASHER_V1 {
        f.push(("dryness", "availableDrynessLevels"));
    }
    f
}
pub fn defaults(id: &str) -> Map<String, Value> {
    let mut a=object_map(json!({"online":true,"laundryState":"idle","laundryPhase":null,"laundryRemainingTime":null,"laundryErrors":[],"program":"Cottons","availablePrograms":["Cottons","Quick"]})).unwrap();
    if id != DRYER_V1 {
        a.extend(object_map(json!({"spinSpeed":"800 rpm","rinse":"normal","availableSpinSpeeds":["no spin","800 rpm"],"availableRinses":["normal","extra"]})).unwrap());
    }
    if id != WASHER_V1 {
        a.extend(
            object_map(
                json!({"dryness":"normal","availableDrynessLevels":["low","normal","extra"]}),
            )
            .unwrap(),
        );
    }
    a
}
pub fn capabilities() -> Vec<String> {
    CAPS.iter().map(|s| s.to_string()).collect()
}
pub fn discovery(id: &str, a: &Map<String, Value>) -> Result<Map<String, Value>> {
    let f = fields(id);
    let mut readable = vec!["phase", "remainingTime", "errors"];
    readable.extend(f.iter().map(|(name, _)| *name));
    let mut out=object_map(json!({"deviceType":match id {WASHER_V1=>"washer",DRYER_V1=>"dryer",_=>"washerDryer"},"supportedStates":readable,"supportedSettings":f.iter().map(|(n,_)|n).collect::<Vec<_>>(),"supportedCommands":["start","pause","resume","stop"],"operatingStates":STATES})).unwrap();
    for (_, catalog) in f {
        let v = value_attr(a, catalog)?;
        let values = v
            .as_array()
            .ok_or_else(|| anyhow!("Invalid laundry catalog"))?;
        ensure!(
            !values.is_empty()
                && values
                    .iter()
                    .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))
                && values
                    .iter()
                    .enumerate()
                    .all(|(i, v)| !values[..i].contains(v)),
            "Invalid laundry catalog"
        );
        out.insert(catalog.into(), v);
    }
    Ok(out)
}
pub fn state(id: &str, a: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    if cap == "data.laundryAttributes" {
        return discovery(id, a);
    }
    if cap == "state.laundrySetting" {
        let mut out = Map::new();
        for (name, catalog) in fields(id) {
            let v = value_attr(a, name)?;
            ensure!(
                v.is_null()
                    || v.as_str()
                        .is_some_and(|s| ensure_in_string_list(a, catalog, s).is_ok()),
                "Invalid laundry setting feedback"
            );
            out.insert(name.into(), v);
        }
        return Ok(out);
    }
    ensure!(cap == "state.laundry", "Unsupported laundry observation");
    let state = value_attr(a, "laundryState")?;
    ensure!(
        state.as_str().is_some_and(|s| STATES.contains(&s)),
        "Invalid laundry task state"
    );
    let phase = value_attr(a, "laundryPhase")?;
    ensure!(
        phase.is_null() || phase.as_str().is_some_and(|s| !s.trim().is_empty()),
        "Invalid laundry phase"
    );
    let remaining = value_attr(a, "laundryRemainingTime")?;
    ensure!(
        remaining.is_null()
            || remaining["_type"] == "Duration"
                && matches!(remaining["unit"].as_str(), Some("sec" | "min"))
                && remaining["value"]
                    .as_f64()
                    .is_some_and(|n| n.is_finite() && n >= 0.0),
        "Invalid laundry remaining duration"
    );
    let errors = value_attr(a, "laundryErrors")?;
    ensure!(
        errors.as_array().is_some_and(|v| v
            .iter()
            .all(|v| v.as_str().is_some_and(|s| !s.trim().is_empty()))),
        "Invalid laundry faults"
    );
    object_map(json!({"state":state,"phase":phase,"remainingTime":remaining,"errors":errors}))
}
pub fn observe(
    id: &str,
    a: &mut Map<String, Value>,
    cap: &str,
    payload: &Map<String, Value>,
) -> Result<()> {
    let pairs: Vec<_> = if cap == "state.laundry" {
        vec![
            ("state", "laundryState"),
            ("phase", "laundryPhase"),
            ("remainingTime", "laundryRemainingTime"),
            ("errors", "laundryErrors"),
        ]
    } else if cap == "state.laundrySetting" {
        fields(id).into_iter().map(|(n, _)| (n, n)).collect()
    } else {
        return Err(anyhow!("Unsupported laundry observation"));
    };
    ensure!(
        !payload.is_empty() && payload.keys().all(|k| pairs.iter().any(|(n, _)| *n == k)),
        "Invalid laundry observation fields"
    );
    let mut next = a.clone();
    copy_optional_payload_fields(&mut next, payload, &pairs);
    state(id, &next, cap)?;
    *a = next;
    Ok(())
}
pub fn command(
    id: &str,
    a: &mut Map<String, Value>,
    cap: &str,
    req: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let mut next = a.clone();
    let observed = if cap == "command.laundrySetting" {
        let f = fields(id);
        ensure!(
            !req.is_empty() && req.keys().all(|k| f.iter().any(|(n, _)| *n == k)),
            "Specify supported laundry settings"
        );
        for (name, catalog) in f {
            if let Some(value) = req.get(name) {
                let value = value
                    .as_str()
                    .ok_or_else(|| anyhow!("Laundry settings require strings"))?;
                ensure_in_string_list(a, catalog, value)?;
                next.insert(name.into(), json!(value));
            }
        }
        "state.laundrySetting"
    } else {
        ensure!(
            cap == "command.laundryControl" && req.len() == 1,
            "Invalid laundry control"
        );
        let current = required_string(a, "laundryState")?;
        let desired = match req.get("command").and_then(Value::as_str) {
            Some("start") => {
                ensure!(
                    ["idle", "stopped", "completed"].contains(&current.as_str()),
                    "Laundry is not ready to start"
                );
                "running"
            }
            Some("pause") => {
                ensure!(current == "running", "Laundry is not running");
                "paused"
            }
            Some("resume") => {
                ensure!(current == "paused", "Laundry is not paused");
                "running"
            }
            Some("stop") => "stopped",
            _ => return Err(anyhow!("Unsupported laundry control")),
        };
        next.insert("laundryState".into(), json!(desired));
        if req.get("command").and_then(Value::as_str) == Some("start") {
            next.insert("laundryPhase".into(), Value::Null);
            next.insert("laundryRemainingTime".into(), Value::Null);
            next.insert("laundryErrors".into(), json!([]));
        }
        if desired == "stopped" {
            next.insert("laundryPhase".into(), Value::Null);
            next.insert("laundryRemainingTime".into(), Value::Null);
        }
        "state.laundry"
    };
    let payload = Value::Object(state(id, &next, observed)?);
    *a = next;
    Ok(CommandOutcome::with_event(observed, payload))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_three_models_keep_settings_separate_and_success_is_readable_state() {
        for id in [WASHER_V1, DRYER_V1, WASHER_DRYER_V1] {
            let mut a = default_attributes(id).unwrap();
            let caps = crate::capabilities(id).unwrap();
            assert_eq!(caps, capabilities());
            assert!(!caps.iter().any(|c| c.starts_with("event.")));
            let facts = discovery(id, &a).unwrap();
            assert_eq!(
                facts["deviceType"],
                if id == WASHER_V1 {
                    "washer"
                } else if id == DRYER_V1 {
                    "dryer"
                } else {
                    "washerDryer"
                }
            );
            let before = a.clone();
            let outcome = handle_command(
                id,
                &mut a,
                "command.laundryControl",
                json!({"command":"start"}).as_object().unwrap(),
            )
            .unwrap();
            assert_eq!(outcome.events[0].capability, "state.laundry");
            assert_eq!(state(id, &a, "state.laundry").unwrap()["state"], "running");
            let full = project_capability_data(id, &a).unwrap();
            assert_eq!(
                full["data.laundryAttributes"]["states"]["operatingStates"],
                json!(STATES)
            );

            assert_eq!(
                state(id, &before, "state.laundrySetting").unwrap(),
                state(id, &a, "state.laundrySetting").unwrap()
            );
            observe(id,&mut a,"state.laundry",json!({"state":"completed","remainingTime":{"_type":"Duration","value":0,"unit":"sec"}}).as_object().unwrap()).unwrap();
            assert_eq!(
                state(id, &a, "state.laundry").unwrap()["state"],
                "completed"
            );
            command(
                id,
                &mut a,
                "command.laundryControl",
                json!({"command":"start"}).as_object().unwrap(),
            )
            .unwrap();
            assert_eq!(state(id, &a, "state.laundry").unwrap()["state"], "running");
        }
    }
    #[test]
    fn mixed_invalid_inputs_do_not_partially_mutate_tasks_or_settings() {
        let mut a = defaults(WASHER_V1);
        let before = a.clone();
        for req in [
            json!({}),
            json!({"program":"Quick","rinse":"bad"}),
            json!({"program":"Quick","dryness":"extra"}),
            json!({"spinSpeed":800}),
        ] {
            assert!(command(
                WASHER_V1,
                &mut a,
                "command.laundrySetting",
                req.as_object().unwrap()
            )
            .is_err());
            assert_eq!(a, before);
        }
        command(
            WASHER_V1,
            &mut a,
            "command.laundrySetting",
            json!({"program":"Quick","spinSpeed":"no spin","rinse":"extra"})
                .as_object()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(a["program"], "Quick");
        assert_eq!(a["laundryState"], "idle");
        for req in [
            json!({"state":"completed","remainingTime":{"_type":"Duration","value":-1,"unit":"sec"}}),
            json!({"state":"invented"}),
            json!({"errors":[1]}),
        ] {
            let old = a.clone();
            assert!(observe(WASHER_V1, &mut a, "state.laundry", req.as_object().unwrap()).is_err());
            assert_eq!(a, old);
        }
    }
}
