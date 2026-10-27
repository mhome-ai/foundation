//! Agent settings needed to choose a valid next operation. Full UI DTOs stay intact.
use serde_json::Value;

pub(super) fn project(target: &str, data: &Value) -> Option<Value> {
    let mut result = data.clone();
    match target {
        "/app/agent/session/get" | "/app/agent/session/save" => {
            retain(&mut result, &["idleTimeoutMinutes", "source"]);
        }
        "/app/agent/llm/summary" | "/app/agent/llm/provider/list" => {
            // providerModels is the authoritative, executable model catalog, including
            // generationControls. providers.models and modelsByUseCase repeat UI data.
            retain(
                &mut result,
                &[
                    "llmMode",
                    "canUseLocalRuntime",
                    "providers",
                    "providerModels",
                    "effectiveMappings",
                    "selectedProviders",
                    "defaultProviders",
                    "llmUseCases",
                ],
            );
            each(&mut result, "providers", |provider| {
                retain(
                    provider,
                    &[
                        "provider",
                        "displayName",
                        "runtime",
                        "useCases",
                        "allowedAgentModes",
                        "authKind",
                        "requiresCredential",
                        "hasCredential",
                        "authenticated",
                        "ready",
                        "status",
                        "reason",
                        "nodeType",
                        "nodeBound",
                        "nodeOnline",
                        "supportedUseCases",
                        "readinessByUseCase",
                    ],
                )
            });
            each(&mut result, "providerModels", |group| {
                each(group, "models", |model| {
                    retain(
                        model,
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
                    )
                });
            });
            mappings(&mut result);
        }
        "/app/agent/llm/mapping/list" => {
            retain(
                &mut result,
                &[
                    "effectiveMappings",
                    "selectedProviders",
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
