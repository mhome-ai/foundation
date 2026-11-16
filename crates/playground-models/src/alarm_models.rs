//! Deterministic endpoint-like alarm, warning-output and security-system models.
use super::*;
use std::collections::BTreeSet;
pub const DEVICE_ALARM_V1: &str = "playground.alarm.device.v1";
pub const SIREN_V1: &str = "playground.alarm.siren.v1";
pub const SECURITY_SYSTEM_V1: &str = "playground.security.system.v1";
const CONDITIONS: &[&str] = &["waterDrainFault", "doorFault"];
const SECURITY_COMMANDS: &[&str] = &["armHome", "armAway", "armNight", "disarm", "trigger"];
const SIREN_COMMANDS: &[&str] = &["on", "off", "sound", "light", "soundAndLight"];
fn object(value: Value) -> Result<Map<String, Value>> {
    value
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("Expected alarm object"))
}
pub fn defaults(model: &str) -> Option<Map<String, Value>> {
    Some(match model {
    DEVICE_ALARM_V1=>json!({"online":true,"alarmStates":{"waterDrainFault":"clear","doorFault":"clear"},"alarmSeverities":{},"enabledAlarms":CONDITIONS,"acknowledgedAlarms":[]}),
    SIREN_V1=>json!({"online":true,"sirenState":"off","sirenMode":"sound","sirenTone":"alarm","sirenVolume":80,"availableSirenTones":["alarm","warning"]}),
    SECURITY_SYSTEM_V1=>json!({"online":true,"securitySystemState":"disarmed"}),
    _=>return None,
}.as_object().unwrap().clone())
}
pub fn capabilities(model: &str) -> Option<&'static [&'static str]> {
    match model {
        DEVICE_ALARM_V1 => Some(&[
            "state.alarm",
            "state.alarmSetting",
            "command.alarmControl",
            "command.alarmSetting",
            "data.alarmAttributes",
            "data.alarmControlAttributes",
        ]),
        SIREN_V1 => Some(&[
            "state.siren",
            "command.sirenControl",
            "data.sirenAttributes",
        ]),
        SECURITY_SYSTEM_V1 => Some(&[
            "state.securitySystem",
            "command.securitySystemControl",
            "data.securitySystemAttributes",
            "data.securitySystemCommandAttributes",
        ]),
        _ => None,
    }
}
pub(crate) fn parameterized_data(cap: &str) -> bool {
    matches!(
        cap,
        "data.alarmControlAttributes" | "data.securitySystemCommandAttributes"
    )
}
fn valid_list(value: Option<&Value>, allowed: &[&str]) -> Result<Vec<String>> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Alarm list is unavailable"))?;
    let mut seen = BTreeSet::new();
    for value in values {
        let name = value
            .as_str()
            .ok_or_else(|| anyhow!("Alarm identifier must be a string"))?;
        ensure!(
            allowed.contains(&name) && seen.insert(name.to_string()),
            "Unsupported or duplicate alarm identifier"
        );
    }
    Ok(seen.into_iter().collect())
}
fn state_value(value: Option<&Value>, allowed: &[&str]) -> Result<Value> {
    let value = value.ok_or_else(|| anyhow!("Alarm state is unavailable"))?;
    ensure!(
        value.is_null() || value.as_str().is_some_and(|s| allowed.contains(&s)),
        "Invalid alarm state"
    );
    Ok(value.clone())
}
fn alarm_snapshot(attrs: &Map<String, Value>) -> Result<Map<String, Value>> {
    let states = attrs
        .get("alarmStates")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Alarm states are unavailable"))?;
    ensure!(
        states.len() == CONDITIONS.len() && states.keys().all(|k| CONDITIONS.contains(&k.as_str())),
        "Alarm snapshot must contain exactly its readable conditions"
    );
    let mut active = false;
    let mut unknown = false;
    let mut records = vec![];
    let severities = attrs
        .get("alarmSeverities")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow!("Alarm severity record is unavailable"))?;
    ensure!(
        severities.keys().all(|k| CONDITIONS.contains(&k.as_str())),
        "Unsupported alarm severity condition"
    );
    for name in CONDITIONS {
        let value = state_value(states.get(*name), &["clear", "active"])?;
        active |= value == "active";
        unknown |= value.is_null();
        let mut record = json!({"alarm":name,"state":value});
        if let Some(severity) = severities.get(*name) {
            ensure!(
                severity.is_null()
                    || matches!(severity.as_str(), Some("info" | "warning" | "critical")),
                "Invalid alarm severity"
            );
            record["severity"] = severity.clone();
        }
        records.push(record);
    }
    object(
        json!({"state":if active{json!("active")}else if unknown{Value::Null}else{json!("clear")},"alarms":records}),
    )
}
pub fn state(
    model: &str,
    attrs: &Map<String, Value>,
    cap: &str,
) -> Option<Result<Map<String, Value>>> {
    let supported = capabilities(model)?;
    if !supported.contains(&cap) {
        return Some(Err(anyhow!("Capability is not exposed by this model")));
    }
    Some(match cap {
        "state.alarm" => alarm_snapshot(attrs),
        "state.alarmSetting" => valid_list(attrs.get("enabledAlarms"), CONDITIONS)
            .and_then(|values| object(json!({"enabledAlarms":values}))),
        "state.siren" => (|| {
            let state = state_value(attrs.get("sirenState"), &["on", "off"])?;
            let mode = state_value(attrs.get("sirenMode"), &["sound", "light", "soundAndLight"])?;
            object(json!({"mode":if state=="on"{mode}else{Value::Null},"state":state}))
        })(),
        "state.securitySystem" => state_value(
            attrs.get("securitySystemState"),
            &[
                "disarmed",
                "arming",
                "armed",
                "armedHome",
                "armedAway",
                "armedNight",
                "armedVacation",
                "armedCustomBypass",
                "disarming",
                "pending",
                "triggered",
            ],
        )
        .and_then(|v| object(json!({"state":v}))),
        "data.alarmAttributes" | "data.sirenAttributes" | "data.securitySystemAttributes" => {
            discovery(model, attrs, cap, "$f1").unwrap()
        }
        cap if cap.starts_with("data.") => Err(anyhow!(
            "This data capability requires operation parameters"
        )),
        _ => Err(anyhow!("Unsupported alarm state capability")),
    })
}
fn siren_tones(attrs: &Map<String, Value>) -> Result<Vec<String>> {
    let values = attrs
        .get("availableSirenTones")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Siren tone catalog is unavailable"))?;
    let mut seen = BTreeSet::new();
    for value in values {
        let tone = value
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Invalid siren tone"))?;
        ensure!(seen.insert(tone.to_owned()), "Ambiguous siren tone catalog");
    }
    ensure!(!seen.is_empty(), "Empty siren tone catalog");
    Ok(seen.into_iter().collect())
}
pub fn discovery(
    model: &str,
    attrs: &Map<String, Value>,
    cap: &str,
    alias: &str,
) -> Option<Result<Map<String, Value>>> {
    let supported = capabilities(model)?;
    if !supported.contains(&cap) {
        return Some(Err(anyhow!("Capability is not exposed by this model")));
    }
    Some((|| match model {
        DEVICE_ALARM_V1 => {
            if (cap == "command.alarmControl" && alias != "$f1")
                || cap == "data.alarmControlAttributes"
            {
                ensure!(
                    ["reset", "acknowledge"].contains(&alias),
                    "Unsupported alarm operation"
                );
                return object(
                    json!({"availableScopes":["all","selected"],"selectableAlarms":CONDITIONS}),
                );
            }
            object(
                json!({"alarms":CONDITIONS,"readableAlarms":CONDITIONS,"configurableAlarms":CONDITIONS,"supportedCommands":["acknowledge","reset"]}),
            )
        }
        SIREN_V1 => {
            let tones = siren_tones(attrs)?;
            object(
                json!({"supportedStates":["mode"],"supportedCommands":SIREN_COMMANDS,"supportedSettings":["tone","volume"],"availableTones":tones}),
            )
        }
        SECURITY_SYSTEM_V1 => {
            if (cap == "command.securitySystemControl" && alias != "$f1")
                || cap == "data.securitySystemCommandAttributes"
            {
                ensure!(
                    SECURITY_COMMANDS.contains(&alias),
                    "Unsupported security operation"
                );
                return object(json!({"codeRequirement":"unsupported"}));
            }
            object(json!({"supportedCommands":SECURITY_COMMANDS}))
        }
        _ => unreachable!(),
    })())
}
pub fn observe(
    model: &str,
    attrs: &mut Map<String, Value>,
    cap: &str,
    payload: &Map<String, Value>,
) -> Option<Result<()>> {
    capabilities(model)?;
    Some((|| {
        let mut staged = attrs.clone();
        match cap {
            "state.alarm" => {
                let records = payload
                    .get("alarms")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow!("Alarm snapshot requires its condition records"))?;
                let mut states = Map::new();
                let mut severity = Map::new();
                for record in records {
                    let name = record["alarm"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Alarm identifier is required"))?;
                    ensure!(
                        CONDITIONS.contains(&name) && !states.contains_key(name),
                        "Unsupported or duplicate alarm identifier"
                    );
                    states.insert(
                        name.into(),
                        record
                            .get("state")
                            .cloned()
                            .ok_or_else(|| anyhow!("Alarm state is required"))?,
                    );
                    if let Some(v) = record.get("severity") {
                        severity.insert(name.into(), v.clone());
                    }
                }
                staged.insert("alarmStates".into(), json!(states));
                staged.insert("alarmSeverities".into(), json!(severity));
                let snapshot = alarm_snapshot(&staged)?;
                ensure!(
                    payload.get("state") == snapshot.get("state"),
                    "Overall alarm state contradicts individual conditions"
                );
            }
            "state.alarmSetting" => {
                valid_list(payload.get("enabledAlarms"), CONDITIONS)?;
                staged.insert("enabledAlarms".into(), payload["enabledAlarms"].clone());
            }
            "state.siren" => {
                staged.insert(
                    "sirenState".into(),
                    payload
                        .get("state")
                        .cloned()
                        .ok_or_else(|| anyhow!("Siren state is required"))?,
                );
                if let Some(mode) = payload.get("mode").filter(|v| !v.is_null()) {
                    staged.insert("sirenMode".into(), mode.clone());
                }
                state(model, &staged, cap).unwrap()?;
            }
            "state.securitySystem" => {
                staged.insert(
                    "securitySystemState".into(),
                    payload
                        .get("state")
                        .cloned()
                        .ok_or_else(|| anyhow!("Security state is required"))?,
                );
                state(model, &staged, cap).unwrap()?;
            }
            _ => return Err(anyhow!("Unsupported alarm observation")),
        }
        *attrs = staged;
        Ok(())
    })())
}
pub fn command(
    model: &str,
    attrs: &mut Map<String, Value>,
    cap: &str,
    req: &Map<String, Value>,
) -> Option<Result<CommandOutcome>> {
    capabilities(model)?;
    Some((|| {
        let action = req.get("command").and_then(Value::as_str).unwrap_or("");
        match cap {
            "command.alarmSetting" => {
                let name = req
                    .get("alarm")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("Alarm condition is required"))?;
                ensure!(CONDITIONS.contains(&name), "Unsupported alarm condition");
                let enabled = req
                    .get("enabledState")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("enabledState is required"))?;
                ensure!(
                    ["enabled", "disabled"].contains(&enabled),
                    "Unsupported enabledState"
                );
                let mut values = valid_list(attrs.get("enabledAlarms"), CONDITIONS)?;
                values.retain(|v| v != name);
                if enabled == "enabled" {
                    values.push(name.into());
                }
                values.sort();
                attrs.insert("enabledAlarms".into(), json!(values));
                Ok(CommandOutcome::with_event(
                    "state.alarmSetting",
                    json!({"enabledAlarms":values}),
                ))
            }
            "command.alarmControl" => {
                ensure!(
                    ["acknowledge", "reset"].contains(&action),
                    "Unsupported alarm operation"
                );
                let selected = match req.get("scope").and_then(Value::as_str) {
                    Some("all") => {
                        ensure!(
                            !req.contains_key("alarms"),
                            "alarms is unavailable for all scope"
                        );
                        CONDITIONS.iter().map(|v| v.to_string()).collect::<Vec<_>>()
                    }
                    Some("selected") => {
                        let values = valid_list(req.get("alarms"), CONDITIONS)?;
                        ensure!(!values.is_empty(), "Selected alarm list is empty");
                        values
                    }
                    _ => return Err(anyhow!("Alarm scope is required")),
                };
                if action == "reset" {
                    let mut states = attrs["alarmStates"]
                        .as_object()
                        .cloned()
                        .ok_or_else(|| anyhow!("Alarm states are unavailable"))?;
                    for name in selected {
                        states.insert(name, json!("clear"));
                    }
                    let mut staged = attrs.clone();
                    staged.insert("alarmStates".into(), json!(states));
                    let snapshot = alarm_snapshot(&staged)?;
                    *attrs = staged;
                    Ok(CommandOutcome::with_event("state.alarm", json!(snapshot)))
                } else {
                    attrs.insert("acknowledgedAlarms".into(), json!(selected));
                    Ok(CommandOutcome::no_response())
                }
            }
            "command.sirenControl" => {
                ensure!(
                    SIREN_COMMANDS.contains(&action),
                    "Unsupported siren command"
                );
                let audible = ["on", "sound", "soundAndLight"].contains(&action);
                ensure!(
                    audible || (!req.contains_key("tone") && !req.contains_key("volume")),
                    "This output command does not accept sound settings"
                );
                let mut staged = attrs.clone();
                if let Some(tone) = req.get("tone") {
                    ensure!(
                        tone.as_str().is_some_and(|value| siren_tones(attrs)
                            .is_ok_and(|tones| tones.iter().any(|tone| tone == value))),
                        "Unsupported siren tone"
                    );
                    staged.insert("sirenTone".into(), tone.clone());
                }
                if let Some(volume) = req.get("volume") {
                    ensure!(
                        volume
                            .as_f64()
                            .is_some_and(|v| v.is_finite() && (0.0..=100.0).contains(&v)),
                        "Invalid siren volume"
                    );
                    staged.insert("sirenVolume".into(), volume.clone());
                }
                staged.insert(
                    "sirenState".into(),
                    json!(if action == "off" { "off" } else { "on" }),
                );
                if ["sound", "light", "soundAndLight"].contains(&action) {
                    staged.insert("sirenMode".into(), json!(action));
                }
                let snapshot = state(model, &staged, "state.siren").unwrap()?;
                *attrs = staged;
                Ok(CommandOutcome::with_event("state.siren", json!(snapshot)))
            }
            "command.securitySystemControl" => {
                ensure!(
                    !req.contains_key("code"),
                    "This security system does not accept an operation code"
                );
                ensure!(
                    SECURITY_COMMANDS.contains(&action),
                    "Unsupported security command"
                );
                let state = match action {
                    "armHome" => "armedHome",
                    "armAway" => "armedAway",
                    "armNight" => "armedNight",
                    "disarm" => "disarmed",
                    "trigger" => "triggered",
                    _ => unreachable!(),
                };
                attrs.insert("securitySystemState".into(), json!(state));
                Ok(CommandOutcome::with_event(
                    "state.securitySystem",
                    json!({"state":state}),
                ))
            }
            _ => Err(anyhow!("Unsupported alarm command capability")),
        }
    })())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(value: Value) -> Map<String, Value> {
        value.as_object().unwrap().clone()
    }
    #[test]
    fn models_have_distinct_complete_snapshots_without_detector_mode_aliases() {
        for model in [DEVICE_ALARM_V1, SIREN_V1, SECURITY_SYSTEM_V1] {
            let device = normalize_instance(DeviceInstance {
                id: model.into(),
                display_name: "Alarm fixture".into(),
                model_id: model.into(),
                attributes: Map::new(),
            })
            .unwrap();
            let snapshot = snapshot_device(&device, vec![]).unwrap();
            assert_eq!(snapshot.model_id, model);
            assert!(!snapshot
                .capability_data
                .contains_key("data.alarmControlAttributes"));
            assert!(!snapshot
                .capability_data
                .contains_key("data.securitySystemCommandAttributes"));
            assert!(snapshot
                .capabilities
                .iter()
                .all(|c| !c.contains("availableModes")));
        }
        let detector = super::super::capabilities(SMOKE_CO_ALARM_V1).unwrap();
        assert!(!detector.contains(&"state.alarm".into()));
        assert!(!default_attributes(SMOKE_CO_ALARM_V1)
            .unwrap()
            .contains_key("alarmMode"));
    }
    #[test]
    fn compound_alarm_updates_reset_only_selected_conditions_and_keep_settings_separate() {
        let mut attrs = default_attributes(DEVICE_ALARM_V1).unwrap();
        apply_observed_payload(DEVICE_ALARM_V1,&mut attrs,"state.alarm",&request(json!({"state":"active","alarms":[{"alarm":"waterDrainFault","state":"active","severity":"critical"},{"alarm":"doorFault","state":"active","severity":"warning"}]}))).unwrap();
        let outcome = handle_command(
            DEVICE_ALARM_V1,
            &mut attrs,
            "command.alarmControl",
            &request(json!({"command":"reset","scope":"selected","alarms":["waterDrainFault"]})),
        )
        .unwrap();
        assert_eq!(outcome.events.len(), 1);
        assert_eq!(outcome.events[0].payload["state"], "active");
        assert_eq!(attrs["alarmStates"]["doorFault"], "active");
        assert_eq!(attrs["alarmStates"]["waterDrainFault"], "clear");
        handle_command(
            DEVICE_ALARM_V1,
            &mut attrs,
            "command.alarmSetting",
            &request(json!({"alarm":"doorFault","enabledState":"disabled"})),
        )
        .unwrap();
        assert_eq!(
            project_state(DEVICE_ALARM_V1, &attrs, "state.alarmSetting").unwrap()["enabledAlarms"],
            json!(["waterDrainFault"])
        );
        assert_eq!(
            project_state(DEVICE_ALARM_V1, &attrs, "state.alarm").unwrap()["state"],
            "active"
        );
        let before = attrs["alarmStates"].clone();
        let acknowledged = handle_command(
            DEVICE_ALARM_V1,
            &mut attrs,
            "command.alarmControl",
            &request(json!({"command":"acknowledge","scope":"all"})),
        )
        .unwrap();
        assert!(acknowledged.events.is_empty());
        assert_eq!(attrs["alarmStates"], before);
        handle_command(
            DEVICE_ALARM_V1,
            &mut attrs,
            "command.alarmControl",
            &request(json!({"command":"reset","scope":"all"})),
        )
        .unwrap();
        assert_eq!(
            project_state(DEVICE_ALARM_V1, &attrs, "state.alarm").unwrap()["state"],
            "clear"
        );
    }
    #[test]
    fn malformed_partial_unknown_and_duplicate_snapshots_do_not_silently_clear_alarms() {
        let mut attrs = default_attributes(DEVICE_ALARM_V1).unwrap();
        for snapshot in [
            json!({"state":"clear","alarms":[{"alarm":"waterDrainFault","state":"active"},{"alarm":"doorFault","state":"clear"}]}),
            json!({"state":"clear","alarms":[{"alarm":"waterDrainFault","state":"clear"}]}),
            json!({"state":"clear","alarms":[{"alarm":"waterDrainFault","state":"clear"},{"alarm":"waterDrainFault","state":"clear"}]}),
            json!({"state":"clear","alarms":[{"alarm":"waterDrainFault","state":"clear","severity":"fatal"},{"alarm":"doorFault","state":"clear"}]}),
        ] {
            let before = attrs.clone();
            assert!(apply_observed_payload(
                DEVICE_ALARM_V1,
                &mut attrs,
                "state.alarm",
                &request(snapshot)
            )
            .is_err());
            assert_eq!(attrs, before);
        }
        for (a, b, overall) in [
            (Value::Null, json!("clear"), Value::Null),
            (Value::Null, json!("active"), json!("active")),
            (json!("clear"), json!("clear"), json!("clear")),
        ] {
            apply_observed_payload(DEVICE_ALARM_V1,&mut attrs,"state.alarm",&request(json!({"state":overall,"alarms":[{"alarm":"waterDrainFault","state":a},{"alarm":"doorFault","state":b}]}))).unwrap();
            assert_eq!(
                project_state(DEVICE_ALARM_V1, &attrs, "state.alarm").unwrap()["state"],
                overall
            );
        }
        for req in [
            json!({"command":"reset","scope":"all","alarms":[]}),
            json!({"command":"reset","scope":"selected","alarms":[]}),
            json!({"command":"reset","scope":"selected","alarms":["doorFault","doorFault"]}),
            json!({"command":"reset","scope":"selected","alarms":["notSupported"]}),
        ] {
            let before = attrs.clone();
            assert!(handle_command(
                DEVICE_ALARM_V1,
                &mut attrs,
                "command.alarmControl",
                &request(req)
            )
            .is_err());
            assert_eq!(attrs, before);
        }
    }
    #[test]
    fn siren_activation_and_security_arming_have_independent_native_effects() {
        let mut siren = default_attributes(SIREN_V1).unwrap();
        let mut system = default_attributes(SECURITY_SYSTEM_V1).unwrap();
        handle_command(
            SIREN_V1,
            &mut siren,
            "command.sirenControl",
            &request(json!({"command":"soundAndLight","tone":"warning","volume":42.5})),
        )
        .unwrap();
        assert_eq!(siren["sirenVolume"], 42.5);
        assert_eq!(
            project_state(SIREN_V1, &siren, "state.siren").unwrap(),
            request(json!({"state":"on","mode":"soundAndLight"}))
        );
        handle_command(
            SECURITY_SYSTEM_V1,
            &mut system,
            "command.securitySystemControl",
            &request(json!({"command":"armAway"})),
        )
        .unwrap();
        assert_eq!(system["securitySystemState"], "armedAway");
        assert_eq!(siren["sirenState"], "on");
        for req in [
            json!({"command":"off","tone":"warning"}),
            json!({"command":"light","volume":50}),
            json!({"command":"on","volume":100.1}),
            json!({"command":"on","tone":"missing"}),
        ] {
            let before = siren.clone();
            assert!(
                handle_command(SIREN_V1, &mut siren, "command.sirenControl", &request(req))
                    .is_err()
            );
            assert_eq!(siren, before);
        }
        let before = system.clone();
        assert!(handle_command(
            SECURITY_SYSTEM_V1,
            &mut system,
            "command.securitySystemControl",
            &request(json!({"command":"arm"}))
        )
        .is_err());
        assert_eq!(system, before);
        assert!(handle_command(
            SECURITY_SYSTEM_V1,
            &mut system,
            "command.sirenControl",
            &request(json!({"command":"on"}))
        )
        .is_err());
        assert_eq!(system, before);
        assert_eq!(
            project_discovery(
                DEVICE_ALARM_V1,
                &default_attributes(DEVICE_ALARM_V1).unwrap(),
                "command.alarmControl",
                "reset"
            )
            .unwrap()["availableScopes"],
            json!(["all", "selected"])
        );
        assert_eq!(
            project_discovery(
                SECURITY_SYSTEM_V1,
                &system,
                "command.securitySystemControl",
                "armAway"
            )
            .unwrap()["codeRequirement"],
            "unsupported"
        );
    }
    #[test]
    fn discovery_and_direct_reads_reject_invalid_tone_catalogs_and_wrong_model_capabilities() {
        let mut attrs = default_attributes(SIREN_V1).unwrap();
        assert_eq!(
            project_state(SIREN_V1, &attrs, "data.sirenAttributes").unwrap(),
            project_discovery(SIREN_V1, &attrs, "state.siren", "$f1").unwrap()
        );
        assert!(project_discovery(SIREN_V1, &attrs, "state.securitySystem", "$f1").is_err());
        assert!(project_state(SIREN_V1, &attrs, "state.alarm").is_err());
        for invalid in [
            Value::Null,
            json!([]),
            json!(["alarm", "alarm"]),
            json!([true]),
            json!([""]),
        ] {
            attrs.insert("availableSirenTones".into(), invalid);
            assert!(project_discovery(SIREN_V1, &attrs, "state.siren", "$f1").is_err());
            let before = attrs.clone();
            assert!(handle_command(
                SIREN_V1,
                &mut attrs,
                "command.sirenControl",
                &request(json!({"command":"sound","tone":"alarm"}))
            )
            .is_err());
            assert_eq!(attrs, before);
        }
    }
}
