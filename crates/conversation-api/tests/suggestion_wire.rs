use conversation_api::execution::wire::{Envelope, Payload, SuggestionPayload};
use serde_json::{Value, json};
fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/execution/suggestion.v1.json")).unwrap()
}
fn schema() -> jsonschema::Validator {
    jsonschema::validator_for(
        &serde_json::from_str::<Value>(include_str!("../schema/execution/envelope.v1.schema.json"))
            .unwrap(),
    )
    .unwrap()
}
#[test]
fn background_generation_uses_the_same_bound_transport_without_a_chat_identity() {
    let input = fixture();
    assert!(schema().is_valid(&input));
    let envelope = Envelope::decode_json(&input.to_string()).unwrap();
    assert_eq!(serde_json::to_value(&envelope).unwrap(), input);
    let Payload::Suggestion(message) = envelope.payload else {
        panic!()
    };
    assert!(matches!(
        message.payload,
        SuggestionPayload::Generate { .. }
    ));
    for field in ["actor", "thread_id", "surface_id", "payer"] {
        assert!(input["payload"]["payload"]["context"].get(field).is_none());
    }
}
#[test]
fn background_transport_rejects_forged_identity_and_unknown_fields() {
    for path in [
        "/payload/payload/context",
        "/payload/payload/payload",
        "/metadata",
    ] {
        let mut value = fixture();
        value.pointer_mut(path).unwrap()["unexpected"] = json!(true);
        assert!(Envelope::decode_json(&value.to_string()).is_err(), "{path}");
        assert!(!schema().is_valid(&value), "{path}");
    }
    for (path, value) in [
        (
            "/payload/payload/context/trigger",
            json!({"type":"SCHEDULED","userId":"forged"}),
        ),
        ("/payload/payload/payload/input", json!([])),
        ("/payload/payload/context/deadlineUnixMs", json!(0)),
    ] {
        let mut input = fixture();
        *input.pointer_mut(path).unwrap() = value;
        assert!(Envelope::decode_json(&input.to_string()).is_err(), "{path}");
        assert!(!schema().is_valid(&input), "{path}");
    }
}
#[test]
fn cancellation_and_failure_are_terminal_messages_not_chat_commands() {
    for payload in [
        json!({"type":"cancel"}),
        json!({"type":"failed","error":{"kind":"unavailable","message":"connection closed","retry_after_ms":null}}),
    ] {
        let mut input = fixture();
        input["payload"]["payload"]["payload"] = payload;
        assert!(schema().is_valid(&input));
        assert!(Envelope::decode_json(&input.to_string()).is_ok());
    }
}

#[test]
fn suggestion_generation_reuses_frozen_model_plan_validation() {
    for (field, value) in [
        ("backend", json!(" ")),
        ("profile", json!("")),
        ("model", json!("")),
        ("context_window_tokens", json!(20_000)),
        ("temperature", json!(-1)),
        ("provider_preferences", json!([""])),
    ] {
        let mut input = fixture();
        input["payload"]["payload"]["payload"]["llm_plan"]["route"][field] = value;
        assert!(
            Envelope::decode_json(&input.to_string()).is_err(),
            "{field}"
        );
    }
}

#[test]
fn nested_route_and_error_fields_remain_closed() {
    for (payload, field) in [
        (
            json!({"type":"failed","error":{"kind":"unavailable","message":"offline","retry_after_ms":null,"unexpected":true}}),
            "error",
        ),
        (
            json!({"type":"generate","input":{},"facts":{},"llm_plan":{}}),
            "generate",
        ),
    ] {
        let mut input = fixture();
        if field == "generate" {
            input["payload"]["payload"]["payload"]["llm_plan"]["route"]["unexpected"] = json!(true);
        } else {
            input["payload"]["payload"]["payload"] = payload;
        }
        assert!(!schema().is_valid(&input));
        assert!(Envelope::decode_json(&input.to_string()).is_err());
    }
}
