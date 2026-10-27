//! Presentation only. Full DTOs, authorization and workflow state are unchanged.
use serde_json::Value;

pub(super) fn project(target: &str, data: &Value) -> Option<Value> {
    if !matches!(
        target,
        "/app/scope/preference/update"
            | "/app/scope/location/update"
            | "/app/host/list"
            | "/app/host/service/install"
            | "/app/host/service/restart"
            | "/app/card-skins/selection/get"
            | "/app/card-skins/catalog/release/list"
            | "/app/card-skins/catalog/release/get"
    ) {
        return None;
    }
    let mut result = data.clone();
    match target {
        "/app/scope/preference/update" => {
            // These are submitted keys, not a comparison with the previous values.
            // Preserve the complete result if an older/malformed DTO lacks the key list.
            if let Some(keys) = data.get("updatedFields").and_then(Value::as_array) {
                if keys.iter().all(Value::is_string) {
                    if let Some(preferences) =
                        result.get_mut("preferences").and_then(Value::as_object_mut)
                    {
                        preferences.retain(|key, _| keys.iter().any(|v| v.as_str() == Some(key)));
                    }
                }
            }
        }
        "/app/scope/location/update" => remove(&mut result, &["previousName", "previousAddress"]),
        "/app/host/list" => {
            if let Some(hosts) = result.get_mut("hosts").and_then(Value::as_array_mut) {
                for host in hosts {
                    if let Some(runtime) = host.pointer_mut("/detail/data/runtime") {
                        // The adjacent component inventory and repairReasons retain the
                        // actionable state. File/link checks and local paths are for Full.
                        remove(runtime, &["runtimeRoot", "components"]);
                    }
                    if let Some(operations) = host
                        .pointer_mut("/detail/data/operations")
                        .and_then(Value::as_array_mut)
                    {
                        for operation in operations {
                            project_operation(operation);
                        }
                    }
                }
            }
        }
        "/app/host/service/install" | "/app/host/service/restart" => project_operation(&mut result),
        "/app/card-skins/selection/get" => {
            if let Some(releases) = result
                .pointer_mut("/selection/releases")
                .and_then(Value::as_array_mut)
            {
                for release in releases {
                    project_release(release, false);
                }
            }
        }
        "/app/card-skins/catalog/release/list" => {
            if let Some(releases) = result.get_mut("releases").and_then(Value::as_array_mut) {
                for release in releases {
                    project_release(release, false);
                }
            }
        }
        "/app/card-skins/catalog/release/get" => project_release(&mut result, true),
        _ => return None,
    }
    Some(result)
}

fn project_operation(operation: &mut Value) {
    if let Some(plan) = operation.pointer_mut("/result/plan") {
        remove(plan, &["catalogs"]);
        if let Some(components) = plan.get_mut("components").and_then(Value::as_array_mut) {
            for component in components {
                remove(component, &["asset", "releaseManifest", "nodeMetadata"]);
            }
        }
    }
}

fn remove(value: &mut Value, fields: &[&str]) {
    if let Some(object) = value.as_object_mut() {
        for field in fields {
            object.remove(*field);
        }
    }
}

fn project_release(release: &mut Value, detail: bool) {
    remove(release, &["baseUrl"]);
    if let Some(manifest) = release.get_mut("manifest") {
        remove(manifest, &["entry", "files"]);
        if !detail {
            if let Some(renderers) = manifest.get_mut("renderers").and_then(Value::as_array_mut) {
                for renderer in renderers {
                    remove(renderer, &["configSchema"]);
                }
            }
        }
    }
}
