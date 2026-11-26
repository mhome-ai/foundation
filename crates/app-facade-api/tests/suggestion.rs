use app_facade_api::suggestion::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn draft_value() -> Value {
    json!({
        "type":"AUTOMATION", "operation":"CREATE",
        "title":"Evening lighting", "summary":"Use a lower evening brightness",
        "reasons":["The recorded manual adjustments suggest a lower setting"],
        "evidence":[{"sourceType":"BEHAVIOR","sourceId":"operation-1",
            "occurredAt":1000,"excerpt":"User adjusted brightness from 80 to 30"}],
        "script":{"script":"name: Evening lighting\n",
            "scripts":{"helper.js":"export const brightness = 30;"},"sdkVersion":"5.0.0"}
    })
}

fn suggestion_value() -> Value {
    let mut value = draft_value();
    value.as_object_mut().unwrap().extend(
        json!({
            "id":"suggestion-1", "spaceId":"space-1", "runId":"run-1", "version":1,
            "status":"PENDING", "createdAt":2000, "updatedAt":2000
        })
        .as_object()
        .unwrap()
        .clone(),
    );
    value
}

#[test]
fn complete_flat_suggestion_roundtrips_with_script_bundle_and_evidence() {
    let value = suggestion_value();
    let result: AutomationSuggestion = serde_json::from_value(value.clone()).unwrap();
    result.validate().unwrap();
    assert_eq!(serde_json::to_value(result).unwrap(), value);
}

#[test]
fn model_draft_cannot_supply_backend_identity_or_lifecycle() {
    for (field, value) in [
        ("id", json!("forged")),
        ("spaceId", json!("other")),
        ("status", json!("RESOLVED")),
        ("version", json!(9)),
        ("payer", json!("owner")),
    ] {
        let mut draft = draft_value();
        draft[field] = value;
        assert!(
            serde_json::from_value::<SuggestionDraft<AutomationContent>>(draft).is_err(),
            "{field}"
        );
    }
}

#[test]
fn create_and_update_cannot_be_confused() {
    let mut value = draft_value();
    value["automationId"] = json!("existing");
    assert!(
        serde_json::from_value::<SuggestionDraft<AutomationContent>>(value.clone())
            .unwrap()
            .validate()
            .is_err()
    );
    value["operation"] = json!("UPDATE");
    assert!(
        serde_json::from_value::<SuggestionDraft<AutomationContent>>(value.clone())
            .unwrap()
            .validate()
            .is_err()
    );
    value["baseScriptHash"] = json!("source-hash");
    serde_json::from_value::<SuggestionDraft<AutomationContent>>(value)
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn incomplete_explanations_and_script_sources_are_rejected() {
    for (pointer, value) in [
        ("/title", json!(" ")),
        ("/reasons", json!([])),
        ("/evidence", json!([])),
        ("/evidence/0/sourceId", json!("")),
        ("/evidence/0/sourceType", json!("behavior")),
        ("/script/script", json!("")),
    ] {
        let mut draft = draft_value();
        *draft.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<SuggestionDraft<AutomationContent>>(draft)
                .unwrap()
                .validate()
                .is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn resolutions_require_actual_backend_outcomes() {
    let mut value = suggestion_value();
    value["status"] = json!("RESOLVED");
    assert!(
        serde_json::from_value::<AutomationSuggestion>(value.clone())
            .unwrap()
            .validate()
            .is_err()
    );
    value["resolution"] = json!({"action":"SAVE", "operationId":"apply-1", "handledAt":3000,
        "actor":{"kind":"user","userId":"user-1"}, "resourceIds":{"automationId":"automation-1"}});
    assert!(
        serde_json::from_value::<AutomationSuggestion>(value.clone())
            .unwrap()
            .validate()
            .is_err()
    );
    value["updatedAt"] = json!(3000);
    serde_json::from_value::<AutomationSuggestion>(value.clone())
        .unwrap()
        .validate()
        .unwrap();
    value["status"] = json!("PENDING");
    assert!(serde_json::from_value::<AutomationSuggestion>(value)
        .unwrap()
        .validate()
        .is_err());
}

#[test]
fn completed_handling_preserves_each_business_outcome_without_reopening() {
    for result in [
        json!({"status":"success", "data":{"automationId":"automation-1", "enabled":true}}),
        json!({"status":"partial", "data":{"automationId":"automation-1", "saved":true},
            "warnings":[{"code":"ENABLE_FAILED", "message":"Saved, but enabling failed"}]}),
        json!({"status":"error", "error":{"code":"INVALID_SCRIPT", "message":"Script validation failed"}}),
        json!({"status":"error", "error":{"code":"OUTCOME_UNKNOWN", "message":"The write outcome is unknown"}}),
        json!({"status":"cancelled"}),
    ] {
        let mut value = suggestion_value();
        value["status"] = json!("RESOLVED");
        value["updatedAt"] = json!(3000);
        value["resolution"] = json!({"action":"APPLY", "operationId":"apply-1", "handledAt":3000,
            "actor":{"kind":"user","userId":"user-1"}, "result":result});
        let suggestion: AutomationSuggestion = serde_json::from_value(value.clone()).unwrap();
        suggestion.validate().unwrap();
        let summary = serde_json::to_value(suggestion.summary()).unwrap();
        assert_eq!(summary["status"], "RESOLVED");
        assert_eq!(summary["outcome"], result["status"]);
        for field in ["resolution", "script", "evidence", "reasons"] {
            assert!(summary.get(field).is_none());
        }
        assert_eq!(serde_json::to_value(suggestion).unwrap(), value);
    }
}

#[test]
fn old_receipts_do_not_invent_a_success_outcome() {
    let mut value = suggestion_value();
    value["status"] = json!("RESOLVED");
    value["resolution"] = json!({"action":"APPLY", "operationId":"apply-1", "handledAt":2000,
        "actor":{"kind":"user","userId":"user-1"}});
    let suggestion: AutomationSuggestion = serde_json::from_value(value.clone()).unwrap();
    suggestion.validate().unwrap();
    assert_eq!(suggestion.summary().outcome, None);
    assert_eq!(serde_json::to_value(suggestion).unwrap(), value);
}

#[test]
fn periodic_settings_default_on_but_save_requires_an_explicit_choice() {
    assert!(SuggestionSettings::default().enabled);
    assert_eq!(SuggestionSettings::default().interval_minutes, 360);
    assert_eq!(SuggestionSettings::default().interval_ms(), 21_600_000);
    assert!(serde_json::from_value::<SuggestionSettings>(json!({})).is_err());
    assert!(
        !serde_json::from_value::<SuggestionSettings>(json!({"enabled":false}))
            .unwrap()
            .enabled
    );
}

#[test]
fn settings_updates_preserve_unspecified_fields_and_accept_existing_callers() {
    let saved = SuggestionSettings {
        enabled: false,
        interval_minutes: 90,
    };
    let enabled: SaveSettingsInput = serde_json::from_value(json!({"enabled":true})).unwrap();
    let updated = enabled.apply_to(saved).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.interval_minutes, 90);
    let interval: SaveSettingsInput =
        serde_json::from_value(json!({"intervalMinutes":360})).unwrap();
    let updated = interval.apply_to(updated).unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.interval_minutes, 360);
    let old: SuggestionSettings = serde_json::from_value(json!({"enabled":false})).unwrap();
    assert!(!old.enabled);
    assert_eq!(old.interval_minutes, 360);
    assert!(SaveSettingsInput::default().apply_to(updated).is_err());
}

// This type is deliberately test-only, not another registered production feature.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE", deny_unknown_fields)]
enum TestContent {
    Information { text: String },
}
impl SuggestionContent for TestContent {
    fn suggestion_type(&self) -> &str {
        "INFORMATION"
    }
    fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Information { text } if !text.is_empty() => Ok(()),
            _ => Err("text required"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum MixedContent {
    Automation(AutomationContent),
    Other(TestContent),
}
impl SuggestionContent for MixedContent {
    fn suggestion_type(&self) -> &str {
        match self {
            Self::Automation(v) => v.suggestion_type(),
            Self::Other(v) => v.suggestion_type(),
        }
    }
    fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Automation(v) => v.validate(),
            Self::Other(v) => v.validate(),
        }
    }
}

#[test]
fn another_kind_uses_the_same_envelope_without_an_automation_script() {
    let mut value = suggestion_value();
    let object = value.as_object_mut().unwrap();
    object.remove("script");
    object.remove("operation");
    object.insert("type".into(), json!("INFORMATION"));
    object.insert("text".into(), json!("A fictional information suggestion"));
    let suggestion: Suggestion<TestContent> = serde_json::from_value(value.clone()).unwrap();
    suggestion.validate().unwrap();
    assert_eq!(serde_json::to_value(suggestion).unwrap(), value);
    assert!(serde_json::from_value::<AutomationSuggestion>(value).is_err());
}

#[test]
fn public_generate_input_cannot_choose_identity_budget_or_trigger() {
    serde_json::from_value::<GenerateInput>(json!({"focus":"Lighting"})).unwrap();
    for field in [
        "spaceId",
        "userId",
        "payer",
        "trigger",
        "budget",
        "allowTools",
    ] {
        assert!(serde_json::from_value::<GenerateInput>(json!({field:"forged"})).is_err());
    }
}

#[test]
fn unknown_content_fields_and_legacy_scope_aliases_are_rejected() {
    for field in ["scopeId", "requestUrl", "executeImmediately"] {
        let mut value = suggestion_value();
        value[field] = json!("unexpected");
        assert!(
            serde_json::from_value::<AutomationSuggestion>(value).is_err(),
            "{field}"
        );
    }
}

fn typed_valid(name: &str, value: Value) -> bool {
    macro_rules! checked {
        ($ty:ty) => {
            serde_json::from_value::<$ty>(value).is_ok_and(|v| v.validate().is_ok())
        };
    }
    match name {
        "AutomationDraft" => checked!(SuggestionDraft<AutomationContent>),
        "AutomationSuggestion" => checked!(AutomationSuggestion),
        "Suggestion" => checked!(Suggestion<TestContent>),
        "GenerationResult" => checked!(SuggestionGenerationResult<MixedContent>),
        "AutomationGenerationResult" => checked!(SuggestionGenerationResult<AutomationContent>),
        "GenerateInput" => checked!(GenerateInput),
        "GenerationGetInput" => checked!(GenerationGetInput),
        "GenerateOutput" => {
            serde_json::from_value::<GenerateOutput>(value).is_ok_and(|v| v.run.validate().is_ok())
        }
        "GenerationGetOutput" => serde_json::from_value::<GenerationGetOutput>(value)
            .is_ok_and(|v| v.run.is_none_or(|r| r.validate().is_ok())),
        "ListInput" => checked!(ListInput),
        "HandleInput" => checked!(HandleInput),
        "FeedbackInput" => checked!(FeedbackInput),
        "SuggestionRun" => checked!(SuggestionRun),
        "SuggestionSettings" => checked!(SuggestionSettings),
        "SaveSettingsInput" => checked!(SaveSettingsInput),
        "SuggestionSummary" => serde_json::from_value::<SuggestionSummary>(value).is_ok(),
        _ => panic!("unhandled fixture {name}"),
    }
}

#[test]
fn shared_corpus_matches_typed_models_and_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/suggestion.v1.schema.json")).unwrap();
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/suggestion.conformance.json")).unwrap();
    for (group, valid) in [("valid", true), ("invalid", false)] {
        for case in corpus[group].as_array().unwrap() {
            let name = case["type"].as_str().unwrap();
            let validator = jsonschema::validator_for(
                &json!({"$defs":schema["$defs"],"$ref":format!("#/$defs/{name}")}),
            )
            .unwrap();
            assert_eq!(validator.is_valid(&case["value"]), valid, "schema {case}");
            assert_eq!(
                typed_valid(name, case["value"].clone()),
                valid,
                "typed {case}"
            );
        }
    }
}

#[test]
fn list_summary_does_not_copy_script_evidence_or_resolution() {
    let suggestion: AutomationSuggestion = serde_json::from_value(suggestion_value()).unwrap();
    let summary = serde_json::to_value(suggestion.summary()).unwrap();
    for field in ["script", "evidence", "reasons", "resolution"] {
        assert!(summary.get(field).is_none());
    }
    assert_eq!(summary["type"], "AUTOMATION");
    assert_eq!(summary["id"], suggestion.id);
    assert_eq!(summary["version"], suggestion.version);
}

#[test]
fn semantic_validation_checks_time_order_and_resolution_space() {
    for (field, time) in [("updatedAt", 1999), ("expiresAt", 2000)] {
        let mut value = suggestion_value();
        value[field] = json!(time);
        assert!(serde_json::from_value::<AutomationSuggestion>(value)
            .unwrap()
            .validate()
            .is_err());
    }
    let mut value = suggestion_value();
    value["status"] = json!("DISMISSED");
    value["resolution"] = json!({"action":"DISMISS", "operationId":"op-2", "handledAt":2000,
        "actor":{"kind":"space","tenantId":"tenant-1","spaceId":"other-space"}});
    assert!(
        serde_json::from_value::<AutomationSuggestion>(value.clone())
            .unwrap()
            .validate()
            .is_err()
    );
    value["resolution"]["actor"]["spaceId"] = json!("space-1");
    serde_json::from_value::<AutomationSuggestion>(value)
        .unwrap()
        .validate()
        .unwrap();
    let run: SuggestionRun = serde_json::from_value(json!({"runId":"r1", "status":"SUCCEEDED",
        "startedAt":2000,"finishedAt":1999,"suggestionIds":[]}))
    .unwrap();
    assert!(run.validate().is_err());
}

#[test]
fn extension_content_cannot_shadow_common_fields_or_misdeclare_its_type() {
    #[derive(Serialize)]
    struct Extension(serde_json::Value);
    impl SuggestionContent for Extension {
        fn suggestion_type(&self) -> &str {
            "TEST"
        }
        fn validate(&self) -> Result<(), &'static str> {
            Ok(())
        }
    }
    let draft: SuggestionDraft<AutomationContent> = serde_json::from_value(draft_value()).unwrap();
    for value in [
        json!({"type":"TEST","title":"shadow"}),
        json!({"type":"TEST","version":5}),
        json!({"type":"OTHER"}),
        json!([]),
    ] {
        let candidate = SuggestionDraft {
            title: draft.title.clone(),
            summary: draft.summary.clone(),
            reasons: draft.reasons.clone(),
            evidence: draft.evidence.clone(),
            expires_at: None,
            content: Extension(value),
        };
        assert!(candidate.validate().is_err());
    }
}

#[test]
fn suggestion_routes_require_space_authority_and_record_only_declared_mutations() {
    use app_facade_api::{
        authorization::{self, OperationPolicy},
        recording,
    };
    for target in TARGETS {
        let write = [GENERATE, HANDLE, FEEDBACK, SETTINGS_SAVE].contains(target);
        assert_eq!(
            authorization::required(target),
            Some(if write {
                OperationPolicy::SpaceWrite
            } else {
                OperationPolicy::SpaceRead
            })
        );
        assert!(!authorization::allows_service(target));
        assert_eq!(recording::selected(target), write);
    }
    let records: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/suggestion.migration.json")).unwrap();
    for record in records {
        match record["kind"].as_str().unwrap() {
            "SUGGESTION" => serde_json::from_value::<AutomationSuggestion>(record["item"].clone())
                .unwrap()
                .validate()
                .unwrap(),
            "STATE" => {
                serde_json::from_value::<SuggestionRun>(record["current"]["run"].clone())
                    .unwrap()
                    .validate()
                    .unwrap();
                serde_json::from_value::<conversation_api::execution::SuggestionInvocationContext>(
                    record["current"]["invocation"].clone(),
                )
                .unwrap()
                .validate()
                .unwrap();
            }
            "SETTINGS" => {
                serde_json::from_value::<SuggestionSettings>(record["settings"].clone())
                    .unwrap()
                    .validate()
                    .unwrap();
            }
            _ => panic!("unknown fixture kind"),
        }
    }
}
