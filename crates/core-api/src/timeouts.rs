//! Static operation budgets shared by execution hosts and callers.
//! Wait profiles only bound receiving a reply; their grace must never extend execution.
use std::time::Duration;

include!("timeouts.generated.rs");

pub fn script_operation(target: &str, payload: &str) -> Option<Operation> {
    let forwarded = FORWARD_PREFIXES
        .iter()
        .find_map(|prefix| target.strip_prefix(prefix));
    let target = forwarded.unwrap_or(target);
    if STORED_SCRIPT_SAVE.contains(&target) {
        return Some(Operation::ScriptSave);
    }
    let saving = SOURCE_SCRIPT_SAVE.contains(&target);
    if !saving && !SOURCE_SCRIPT_VALIDATION.contains(&target) {
        return None;
    }
    let root: serde_json::Value = serde_json::from_str(payload).ok()?;
    let input = if forwarded.is_some() {
        root.get("call")?.get("input")?
    } else if target.starts_with("/app/") {
        root.get("input")?
    } else {
        &root
    };
    let scripts = input.get("script")?.get("scripts")?.as_object()?;
    (!scripts.is_empty()).then_some(if saving {
        Operation::ScriptSave
    } else {
        Operation::ScriptValidation
    })
}

pub fn ordinary_operation(target: &str) -> Operation {
    ORDINARY_OVERRIDES
        .iter()
        .find(|(value, _)| *value == target)
        .map_or(Operation::CoreRequest, |(_, operation)| *operation)
}

pub fn request_operation(target: &str, payload: &str) -> Operation {
    script_operation(target, payload).unwrap_or_else(|| ordinary_operation(target))
}

pub fn reply_timeout(operation: Operation, profile: WaitProfile) -> Duration {
    operation.timeout() + profile.grace(operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_selection_and_wait_fixtures() {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../fixtures/operation-timeouts.conformance.json"
        ))
        .unwrap();
        for case in cases.as_array().unwrap() {
            let operation = request_operation(
                case["target"].as_str().unwrap(),
                &case["payload"].to_string(),
            );
            assert_eq!(
                operation.id(),
                case["operation"].as_str().unwrap(),
                "{case}"
            );
            assert_eq!(
                operation.timeout().as_millis(),
                case["totalMs"].as_u64().unwrap() as u128
            );
            if let Some(wait) = case["lionWaitMs"].as_u64() {
                assert_eq!(
                    reply_timeout(operation, WaitProfile::LionForward).as_millis(),
                    wait as u128
                );
            }
        }
    }
}
