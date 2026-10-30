//! Agent settings needed to choose a valid next operation. Full UI DTOs stay intact.
use serde_json::Value;

pub(super) fn project(target: &str, data: &Value) -> Option<Value> {
    let mut result = data.clone();
    match target {
        "/app/agent/session/get" | "/app/agent/session/save" => {
            retain(&mut result, &["idleTimeoutMinutes", "source"]);
        }
        "/app/agent/llm/summary" | "/app/agent/llm/provider/list" => {
            // Keep the actionable model catalog and controls; catalog metadata stays in Full.
            retain(
                &mut result,
                &[
                    "llmMode",
                    "providers",
                    "providerModels",
                    "effectiveMappings",
                    "defaultProviders",
                    "llmUseCases",
                ],
            );
            each(&mut result, "providers", provider);
            each(&mut result, "providerModels", |group| {
                each(group, "models", model)
            });
            mappings(&mut result);
        }
        "/app/agent/llm/provider/get" => {
            if let Some(value) = result.get_mut("provider") {
                provider(value);
            }
            each(&mut result, "models", model);
        }
        "/app/agent/llm/mapping/list" => {
            retain(
                &mut result,
                &[
                    "llmMode",
                    "effectiveMappings",
                    "defaultProviders",
                    "llmUseCases",
                ],
            );
            mappings(&mut result);
        }
        _ => return None,
    }
    Some(result)
}

fn mappings(value: &mut Value) {
    // Keep configuredModel, effective model, source and every generation override.
    // They explain fallbacks and let callers avoid accidentally replacing settings.
    each(value, "defaultProviders", |provider| {
        retain(provider, &["useCase", "llmMode", "provider"])
    });
    each(value, "llmUseCases", |use_case| {
        retain(
            use_case,
            &[
                "useCase",
                "displayName",
                "description",
                "configurable",
                "active",
                "modes",
            ],
        );
        each(use_case, "modes", |mode| {
            retain(
                mode,
                &["mode", "displayName", "description", "requirements"],
            )
        });
    });
}

fn retain(value: &mut Value, keys: &[&str]) {
    if let Some(fields) = value.as_object_mut() {
        fields.retain(|key, _| keys.contains(&key.as_str()));
    }
}

fn each(value: &mut Value, key: &str, visit: impl Fn(&mut Value)) {
    if let Some(items) = value.get_mut(key).and_then(Value::as_array_mut) {
        for item in items {
            visit(item);
        }
    }
}

fn provider(value: &mut Value) {
    retain(
        value,
        &[
            "provider",
            "displayName",
            "runtime",
            "useCases",
            "allowedAgentModes",
            "authKind",
            "configuration",
        ],
    );
    if let Some(config) = value.get_mut("configuration") {
        retain(
            config,
            &[
                "requiresCredential",
                "hasCredential",
                "ready",
                "status",
                "reason",
                "nodeType",
                "nodeBound",
                "nodeOnline",
                "supportedUseCases",
                "readinessByUseCase",
            ],
        );
    }
}
fn model(value: &mut Value) {
    retain(
        value,
        &[
            "id",
            "displayName",
            "modelType",
            "available",
            "deprecated",
            "successor",
            "supportsTools",
            "supportsVision",
            "contextWindowTokens",
            "inputModalities",
            "outputModalities",
            "generationControls",
        ],
    );
}
