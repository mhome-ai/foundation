//! A doorbell sounder with independent playback, persistent settings and ring events.
use super::*;
use std::collections::BTreeSet;
pub const CHIME_V1: &str = "playground.chime.doorbell.v1";
const CAPS: &[&str] = &[
    "state.chime",
    "state.chimeSetting",
    "command.chimeControl",
    "command.chimeSetting",
    "event.chime",
    "data.chimeAttributes",
];
pub fn defaults() -> Map<String, Value> {
    object_map(json!({"online":true,"chimeState":"idle","chimeEnabledState":"enabled","chimeTone":"Ding dong","availableChimeTones":["Ding dong","Bell"]})).unwrap()
}
pub fn capabilities() -> Vec<String> {
    CAPS.iter().map(|s| (*s).to_string()).collect()
}
fn tones(attrs: &Map<String, Value>) -> Result<Vec<String>> {
    let values = attrs
        .get("availableChimeTones")
        .and_then(Value::as_array)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("Chime tone catalog is unavailable"))?;
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|value| {
            let name = value
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| anyhow!("Invalid chime tone"))?;
            ensure!(seen.insert(name), "Duplicate chime tone");
            Ok(name.to_string())
        })
        .collect()
}
fn reading(attrs: &Map<String, Value>, field: &str, choices: &[&str]) -> Result<Value> {
    let value = attrs
        .get(field)
        .ok_or_else(|| anyhow!("Chime feedback is unavailable"))?;
    ensure!(
        value.is_null() || value.as_str().is_some_and(|v| choices.contains(&v)),
        "Invalid chime feedback: {field}"
    );
    Ok(value.clone())
}
pub fn discovery(attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    ensure!(CAPS.contains(&cap), "Unsupported chime capability");
    if cap == "state.chime" {
        return Ok(Map::new());
    }
    object_map(
        json!({"supportedStates":["enabledState","tone"],"supportedSettings":["enabledState","tone"],"supportedCommands":["play","stop"],"canPlayTone":true,"availableTones":tones(attrs)?,"supportedChimeTypes":[]}),
    )
}
pub fn state(attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    match cap {
        "state.chime" => {
            object_map(json!({"state":reading(attrs,"chimeState",&["playing","idle"])?}))
        }
        "state.chimeSetting" => {
            let catalog = tones(attrs)?;
            let tone = attrs
                .get("chimeTone")
                .ok_or_else(|| anyhow!("Chime default tone is unavailable"))?;
            ensure!(
                tone.is_null()
                    || tone
                        .as_str()
                        .is_some_and(|v| catalog.iter().any(|name| name == v)),
                "Default chime tone is outside its catalog"
            );
            object_map(
                json!({"enabledState":reading(attrs,"chimeEnabledState",&["enabled","disabled"])?,"tone":tone}),
            )
        }
        "data.chimeAttributes" => discovery(attrs, cap),
        // Occurrences are delivered only by command/observation events, not synthesized
        // from a cached boolean when a snapshot is read or a subscription starts.
        "event.chime" => Ok(Map::new()),
        _ => Err(anyhow!("Unsupported chime state capability")),
    }
}
pub fn observe(
    attrs: &mut Map<String, Value>,
    cap: &str,
    payload: &Map<String, Value>,
) -> Result<()> {
    let mut next = attrs.clone();
    match cap {
        "state.chime" => {
            ensure!(
                payload.keys().all(|k| k == "state") && payload.contains_key("state"),
                "Invalid chime observation"
            );
            next.insert("chimeState".into(), payload["state"].clone());
        }
        "state.chimeSetting" => {
            ensure!(
                !payload.is_empty()
                    && payload
                        .keys()
                        .all(|k| ["enabledState", "tone"].contains(&k.as_str())),
                "Invalid chime setting observation"
            );
            for (public, internal) in [("enabledState", "chimeEnabledState"), ("tone", "chimeTone")]
            {
                if let Some(value) = payload.get(public) {
                    next.insert(internal.into(), value.clone());
                }
            }
        }
        "event.chime" => {
            ensure!(
                payload.len() == 1 && payload.get("occurred") == Some(&Value::Bool(true)),
                "A chime event must represent one occurrence"
            );
            return Ok(());
        }
        _ => return Err(anyhow!("Unsupported chime observation")),
    }
    state(&next, cap)?;
    *attrs = next;
    Ok(())
}
pub fn command(
    attrs: &mut Map<String, Value>,
    cap: &str,
    request: &Map<String, Value>,
) -> Result<CommandOutcome> {
    let setting = cap == "command.chimeSetting";
    ensure!(
        setting || cap == "command.chimeControl",
        "Unsupported chime command"
    );
    let allowed = if setting {
        vec!["device", "enabledState", "tone"]
    } else {
        vec!["device", "command", "tone"]
    };
    ensure!(
        request.keys().all(|k| allowed.contains(&k.as_str())),
        "Unexpected chime parameter"
    );
    let request: Map<String, Value> = request
        .iter()
        .filter(|(key, value)| {
            !["tone", "enabledState"].contains(&key.as_str()) || !value.is_null()
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let request = &request;
    let catalog = tones(attrs)?;
    if let Some(tone) = request.get("tone") {
        ensure!(
            tone.as_str()
                .is_some_and(|name| catalog.iter().any(|v| v == name)),
            "Unsupported chime tone"
        );
    }
    if setting {
        ensure!(
            request.contains_key("enabledState") || request.contains_key("tone"),
            "At least one chime setting is required"
        );
        let payload = request
            .iter()
            .filter(|(key, _)| key.as_str() != "device")
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        // Null is valid unknown feedback, but is not a request to clear a setting.
        ensure!(
            request
                .get("enabledState")
                .is_none_or(|v| matches!(v.as_str(), Some("enabled" | "disabled"))),
            "Invalid chime enabled setting"
        );
        observe(attrs, "state.chimeSetting", &payload)?;
        return Ok(CommandOutcome::with_event(
            "state.chimeSetting",
            Value::Object(state(attrs, "state.chimeSetting")?),
        ));
    }
    match required_string(request, "command")?.as_str() {
        "stop" => {
            ensure!(
                !request.contains_key("tone"),
                "Stopping a chime does not accept a tone"
            );
            attrs.insert("chimeState".into(), json!("idle"));
            Ok(CommandOutcome::with_event(
                "state.chime",
                json!({"state":"idle"}),
            ))
        }
        "play" => {
            ensure!(
                attrs.get("chimeEnabledState") == Some(&json!("enabled")),
                "Chime is disabled or its enabled setting is unknown"
            );
            let tone = request.get("tone").or_else(|| attrs.get("chimeTone"));
            ensure!(
                tone.and_then(Value::as_str)
                    .is_some_and(|name| catalog.iter().any(|v| v == name)),
                "Chime playback tone is unknown"
            );
            // A deterministic short playback completes within this command. Tests can
            // inject native playing feedback independently; no timer guesses its duration.
            attrs.insert("chimeState".into(), json!("idle"));
            Ok(CommandOutcome {
                response: Value::Null,
                events: vec![
                    CommandEvent {
                        capability: "state.chime".into(),
                        payload: json!({"state":"playing"}),
                    },
                    CommandEvent {
                        capability: "event.chime".into(),
                        payload: json!({"occurred":true}),
                    },
                    CommandEvent {
                        capability: "state.chime".into(),
                        payload: json!({"state":"idle"}),
                    },
                ],
            })
        }
        _ => Err(anyhow!("Unsupported chime playback command")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn req(value: Value) -> Map<String, Value> {
        object_map(value).unwrap()
    }
    #[test]
    fn complete_playbacks_repeat_without_changing_settings_or_leaking_to_detector_models() {
        let mut attrs = default_attributes(CHIME_V1).unwrap();
        for _ in 0..2 {
            let before = state(&attrs, "state.chimeSetting").unwrap();
            let result = handle_command(
                CHIME_V1,
                &mut attrs,
                "command.chimeControl",
                &req(json!({"command":"play","tone":"Bell"})),
            )
            .unwrap();
            assert_eq!(
                result
                    .events
                    .iter()
                    .map(|e| e.capability.as_str())
                    .collect::<Vec<_>>(),
                vec!["state.chime", "event.chime", "state.chime"]
            );
            assert_eq!(result.events[1].payload, json!({"occurred":true}));
            assert_eq!(state(&attrs, "state.chimeSetting").unwrap(), before);
            assert_eq!(state(&attrs, "state.chime").unwrap()["state"], "idle");
        }
        assert!(!crate::capabilities(SMOKE_CO_ALARM_V1)
            .unwrap()
            .iter()
            .any(|c| c.contains("chime")));
        assert_eq!(
            project_capability_data(CHIME_V1, &attrs).unwrap()["event.chime"]["states"],
            json!({})
        );
    }
    #[test]
    fn settings_stopping_unknowns_and_invalid_requests_preserve_independent_contracts() {
        let mut attrs = defaults();
        let changed = command(
            &mut attrs,
            "command.chimeSetting",
            &req(json!({"tone":"Bell","enabledState":"disabled"})),
        )
        .unwrap();
        assert_eq!(changed.events.len(), 1);
        assert_eq!(changed.events[0].capability, "state.chimeSetting");
        let before = attrs.clone();
        let default_play = command(
            &mut attrs,
            "command.chimeControl",
            &req(json!({"command":"stop","tone":null})),
        )
        .unwrap();
        assert!(default_play
            .events
            .iter()
            .all(|event| event.capability != "event.chime"));
        for (cap, request) in [
            (
                "command.chimeSetting",
                json!({"tone":"missing","enabledState":"enabled"}),
            ),
            ("command.chimeSetting", json!({"enabledState":null})),
            ("command.chimeSetting", json!({})),
            ("command.chimeControl", json!({"command":"play"})),
            (
                "command.chimeControl",
                json!({"command":"stop","tone":"Bell"}),
            ),
            ("command.chimeControl", json!({"command":"on"})),
        ] {
            assert!(command(&mut attrs, cap, &req(request)).is_err());
            assert_eq!(attrs, before);
        }
        let stopped = command(
            &mut attrs,
            "command.chimeControl",
            &req(json!({"command":"stop"})),
        )
        .unwrap();
        assert!(stopped.events.iter().all(|e| e.capability != "event.chime"));
        assert_eq!(
            state(&attrs, "state.chimeSetting").unwrap()["enabledState"],
            "disabled"
        );
        observe(
            &mut attrs,
            "state.chimeSetting",
            &req(json!({"enabledState":null,"tone":null})),
        )
        .unwrap();
        assert_eq!(
            state(&attrs, "state.chimeSetting").unwrap(),
            req(json!({"enabledState":null,"tone":null}))
        );
        let before = attrs.clone();
        assert!(observe(
            &mut attrs,
            "state.chimeSetting",
            &req(json!({"enabledState":"enabled","tone":"missing"}))
        )
        .is_err());
        assert_eq!(attrs, before);
        assert!(observe(&mut attrs, "event.chime", &req(json!({"occurred":false}))).is_err());
    }
}
