//! Media fixtures distinguish full feedback, command-only remotes and read-only applications.
use super::*;
use anyhow::bail;
pub const TV_V1: &str = "playground.media.television.v1";
pub const CHANNEL_REMOTE_V1: &str = "playground.media.channel-remote.v1";
pub const APP_OBSERVER_V1: &str = "playground.media.application-observer.v1";
pub fn supported(model: &str) -> bool {
    [TV_V1, CHANNEL_REMOTE_V1, APP_OBSERVER_V1].contains(&model)
}
pub fn capabilities(model: &str) -> Vec<String> {
    let mut out = vec![];
    if model != APP_OBSERVER_V1 {
        out.extend(["command.controlMediaChannel", "data.mediaChannelAttributes"]);
    }
    if model == TV_V1 {
        out.push("state.mediaChannel");
        out.push("command.launchMediaApplication");
    }
    if model != CHANNEL_REMOTE_V1 {
        out.extend(["state.mediaApplication", "data.mediaApplicationAttributes"]);
    }
    out.into_iter().map(String::from).collect()
}
pub fn defaults(model: &str) -> Map<String, Value> {
    let mut out = object_map(json!({"online":true})).unwrap();
    if model != APP_OBSERVER_V1 {
        out.extend(object_map(json!({"selectableNumbers":["7.1","7.10"]})).unwrap());
    }
    if model == TV_V1 {
        out.extend(object_map(json!({"number":"7.1","name":"News","selectableNames":["News","Sports"],"reportableNumbers":["7.1","7.10","99"],"reportableNames":["News","Sports","Feedback only"],"selectableApplicationIds":["app.video","app.music"]})).unwrap());
    }
    if model != CHANNEL_REMOTE_V1 {
        out.extend(object_map(json!({"applicationId":"app.video","applicationName":"Video","activity":"foreground","reportableApplicationIds":["app.video","app.music","app.system"]})).unwrap());
    }
    out
}
fn application(cap: &str) -> bool {
    cap.contains("Application")
}
fn catalog(attrs: &Map<String, Value>, name: &str) -> Result<Option<Vec<String>>> {
    let Some(v) = attrs.get(name).filter(|v| !v.is_null()) else {
        return Ok(None);
    };
    let list = v
        .as_array()
        .ok_or_else(|| anyhow!("Invalid media catalog {name}"))?;
    let mut out = vec![];
    for v in list {
        let s = v
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow!("Invalid media catalog item {name}"))?;
        ensure!(!out.iter().any(|v| v == s), "Duplicate media catalog item");
        out.push(s.to_owned());
    }
    Ok(Some(out))
}
pub fn discovery(model: &str, attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    ensure!(
        capabilities(model).iter().any(|c| c == cap),
        "Unsupported media capability"
    );
    let app = application(cap);
    let mut out = if app {
        object_map(
            json!({"supportedStates":["applicationId","name","activity"],"activityStates":["foreground","hidden","stopped","none"]}),
        )?
    } else {
        object_map(
            json!({"supportedStates":if model==TV_V1 {vec!["number","name"]}else{vec![]},"supportedCommands":if model==TV_V1 {vec!["selectByNumber","selectByName","next","previous"]}else{vec!["selectByNumber","next","previous"]}}),
        )?
    };
    let keys = if app {
        vec!["reportableApplicationIds", "selectableApplicationIds"]
    } else {
        vec![
            "reportableNumbers",
            "selectableNumbers",
            "reportableNames",
            "selectableNames",
        ]
    };
    for key in keys {
        if (model == CHANNEL_REMOTE_V1
            && (key.starts_with("reportable") || key == "selectableNames"))
            || (key.starts_with("selectable") && model == APP_OBSERVER_V1)
        {
            continue;
        }
        if let Some(values) = catalog(attrs, key)? {
            out.insert(key.into(), json!(values));
        }
    }
    Ok(out)
}
pub fn state(model: &str, attrs: &Map<String, Value>, cap: &str) -> Result<Map<String, Value>> {
    if cap.starts_with("data.") {
        return discovery(model, attrs, cap);
    }
    ensure!(
        cap.starts_with("state.") && capabilities(model).iter().any(|c| c == cap),
        "Unsupported media feedback"
    );
    let fields = if application(cap) {
        vec![
            ("applicationId", "applicationId"),
            ("name", "applicationName"),
            ("activity", "activity"),
        ]
    } else {
        vec![("number", "number"), ("name", "name")]
    };
    let mut out = Map::new();
    for (field, key) in fields {
        let v = attrs
            .get(key)
            .ok_or_else(|| anyhow!("Media observation {field} is unavailable"))?;
        ensure!(
            v.is_null() || v.as_str().is_some_and(|s| !s.is_empty()),
            "Invalid media observation"
        );
        let key = match field {
            "number" => Some("reportableNumbers"),
            "name" if application(cap) => None,
            "name" => Some("reportableNames"),
            "applicationId" => Some("reportableApplicationIds"),
            _ => None,
        };
        if let Some(value) = v.as_str() {
            if let Some(key) = key {
                if let Some(values) = catalog(attrs, key)? {
                    ensure!(
                        values.iter().any(|v| v == value),
                        "Media feedback is outside its reportable catalog"
                    );
                }
            }
            if field == "activity" {
                ensure!(
                    ["foreground", "hidden", "stopped", "none"].contains(&value),
                    "Invalid application activity"
                );
            }
        }
        out.insert(field.into(), v.clone());
    }
    if application(cap) && attrs.get("activity") == Some(&json!("none")) {
        ensure!(
            attrs.get("applicationId") == Some(&Value::Null)
                && attrs.get("applicationName") == Some(&Value::Null),
            "No active application cannot retain an application identity"
        );
    }
    Ok(out)
}
pub fn observe(
    model: &str,
    attrs: &mut Map<String, Value>,
    cap: &str,
    payload: &Map<String, Value>,
) -> Result<()> {
    ensure!(
        cap.starts_with("state.") && capabilities(model).iter().any(|c| c == cap),
        "Unsupported media feedback"
    );
    let allowed = if application(cap) {
        vec!["applicationId", "name", "activity"]
    } else {
        vec!["number", "name"]
    };
    ensure!(
        !payload.is_empty() && payload.keys().all(|k| allowed.contains(&k.as_str())),
        "Invalid media observation fields"
    );
    let mut next = attrs.clone();
    for (k, v) in payload {
        next.insert(
            if application(cap) && k == "name" {
                "applicationName"
            } else {
                k
            }
            .into(),
            v.clone(),
        );
    }
    state(model, &next, cap)?;
    *attrs = next;
    Ok(())
}
fn selectable(attrs: &Map<String, Value>, key: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "Media selection must be nonempty");
    if let Some(values) = catalog(attrs, key)? {
        ensure!(
            values.iter().any(|v| v == value),
            "Media selection is unavailable"
        );
    }
    Ok(())
}
pub fn command(
    model: &str,
    attrs: &mut Map<String, Value>,
    cap: &str,
    req: &Map<String, Value>,
) -> Result<CommandOutcome> {
    ensure!(
        cap.starts_with("command.") && capabilities(model).iter().any(|c| c == cap),
        "Unsupported media command"
    );
    let mut next = attrs.clone();
    if cap == "command.launchMediaApplication" {
        ensure!(
            req.keys()
                .all(|k| ["device", "applicationId"].contains(&k.as_str())),
            "Unexpected media field"
        );
        let id = req
            .get("applicationId")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Application ID is required"))?;
        selectable(attrs, "selectableApplicationIds", id)?;
        next.insert("applicationId".into(), json!(id));
        next.insert("applicationName".into(), json!(id));
        next.insert("activity".into(), json!("foreground"));
    } else {
        let cmd = req
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow!("Channel command is required"))?;
        let field = match cmd {
            "selectByNumber" => Some("number"),
            "selectByName" if model == TV_V1 => Some("name"),
            "next" | "previous" => None,
            _ => bail!("Unsupported channel command"),
        };
        ensure!(
            req.keys()
                .all(|k| k == "device" || k == "command" || field == Some(k.as_str())),
            "Unexpected channel field"
        );
        let numbers = catalog(attrs, "selectableNumbers")?;
        let names = catalog(attrs, "selectableNames")?;
        let (number, name) = if let Some(field) = field {
            let v = req
                .get(field)
                .and_then(Value::as_str)
                .ok_or_else(|| anyhow!("Channel {field} is required"))?;
            selectable(
                attrs,
                if field == "number" {
                    "selectableNumbers"
                } else {
                    "selectableNames"
                },
                v,
            )?;
            if field == "number" {
                (
                    json!(v),
                    numbers
                        .as_ref()
                        .and_then(|ns| ns.iter().position(|n| n == v))
                        .and_then(|i| names.as_ref().and_then(|ns| ns.get(i)))
                        .map_or(Value::Null, |n| json!(n)),
                )
            } else {
                (
                    names
                        .as_ref()
                        .and_then(|ns| ns.iter().position(|n| n == v))
                        .and_then(|i| numbers.as_ref().and_then(|ns| ns.get(i)))
                        .map_or(Value::Null, |n| json!(n)),
                    json!(v),
                )
            }
        } else {
            let ns = numbers
                .as_ref()
                .filter(|ns| !ns.is_empty())
                .ok_or_else(|| anyhow!("Channel order is unavailable"))?;
            let current = attrs
                .get("number")
                .and_then(Value::as_str)
                .and_then(|n| ns.iter().position(|v| v == n))
                .unwrap_or(0);
            let i = if cmd == "next" {
                (current + 1) % ns.len()
            } else {
                (current + ns.len() - 1) % ns.len()
            };
            (
                json!(ns[i]),
                names
                    .as_ref()
                    .and_then(|ns| ns.get(i))
                    .map_or(Value::Null, |n| json!(n)),
            )
        };
        next.insert("number".into(), number);
        next.insert("name".into(), name);
    }
    if model == CHANNEL_REMOTE_V1 {
        return Ok(CommandOutcome::no_response());
    }
    let state_cap = if cap == "command.launchMediaApplication" {
        "state.mediaApplication"
    } else {
        "state.mediaChannel"
    };
    let payload = Value::Object(state(model, &next, state_cap)?);
    *attrs = next;
    Ok(CommandOutcome::with_event(state_cap, payload))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn channels_apps_and_command_only_remotes_have_independent_contracts_and_atomic_effects() {
        let mut attrs = default_attributes(TV_V1).unwrap();
        let feedback = project_state(TV_V1, &attrs, "state.mediaChannel").unwrap();
        assert_eq!(feedback["number"], "7.1");
        let result = handle_command(
            TV_V1,
            &mut attrs,
            "command.controlMediaChannel",
            &object_map(json!({"command":"selectByNumber","number":"7.10"})).unwrap(),
        )
        .unwrap();
        assert_eq!(result.events[0].payload["number"], "7.10");
        let prior = attrs.clone();
        assert!(handle_command(
            TV_V1,
            &mut attrs,
            "command.controlMediaChannel",
            &object_map(json!({"command":"next","number":"7"})).unwrap()
        )
        .is_err());
        assert_eq!(attrs, prior);
        let result = handle_command(
            TV_V1,
            &mut attrs,
            "command.launchMediaApplication",
            &object_map(json!({"applicationId":"app.music"})).unwrap(),
        )
        .unwrap();
        assert_eq!(result.events[0].payload["applicationId"], "app.music");
        let mut remote = default_attributes(CHANNEL_REMOTE_V1).unwrap();
        let prior = remote.clone();
        assert!(handle_command(
            CHANNEL_REMOTE_V1,
            &mut remote,
            "command.controlMediaChannel",
            &object_map(json!({"command":"next"})).unwrap()
        )
        .unwrap()
        .events
        .is_empty());
        assert_eq!(remote, prior);
        assert!(!capabilities(CHANNEL_REMOTE_V1)
            .iter()
            .any(|c| c.starts_with("state.")));
        assert!(!capabilities(APP_OBSERVER_V1)
            .iter()
            .any(|c| c.starts_with("command.")));
    }
    #[test]
    fn empty_and_missing_catalogs_and_read_only_values_are_preserved() {
        let mut attrs = defaults(TV_V1);
        attrs.insert("selectableApplicationIds".into(), json!([]));
        let prior = attrs.clone();
        assert!(command(
            TV_V1,
            &mut attrs,
            "command.launchMediaApplication",
            &object_map(json!({"applicationId":"app.video"})).unwrap()
        )
        .is_err());
        assert_eq!(attrs, prior);
        attrs.insert("selectableApplicationIds".into(), Value::Null);
        assert!(!discovery(TV_V1, &attrs, "data.mediaApplicationAttributes")
            .unwrap()
            .contains_key("selectableApplicationIds"));
        command(
            TV_V1,
            &mut attrs,
            "command.launchMediaApplication",
            &object_map(json!({"applicationId":"app.system"})).unwrap(),
        )
        .unwrap();
        observe(
            TV_V1,
            &mut attrs,
            "state.mediaChannel",
            &object_map(json!({"number":"99","name":"Feedback only"})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            state(TV_V1, &attrs, "state.mediaChannel").unwrap()["number"],
            "99"
        );
        assert!(command(
            TV_V1,
            &mut attrs,
            "command.controlMediaChannel",
            &object_map(json!({"command":"selectByNumber","number":"99"})).unwrap()
        )
        .is_err());
    }
}
