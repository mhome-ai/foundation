use conversation_api::{
    ConversationAccessMode, ConversationEvent, ConversationQueue, DebugEvent,
    InteractionAnswerDisposition, InteractionAnswerRequest, InteractionAnswerResponse,
    InteractionSubmitDisposition, InteractionSubmitRequest, InteractionSubmitResponse,
    MessageEnqueueDisposition, MessageEnqueueRequest, MessageEnqueueResponse, MessagePart,
    QueueReorderRequest, RequestCancelOutcome, RequestCancelPhase, RequestCancelRequest,
    RequestCancelResponse, SessionStartDisposition, SessionStartRequest, SessionStartResponse,
    ThreadArchiveRequest, ThreadCatalog, ThreadCreateRequest, ThreadListRequest, ThreadLoadRequest,
    ThreadLoadResponse, ThreadRotateRequest, TurnSessionDisposition, TurnSubmitDisposition,
    TurnSubmitRequest, TurnSubmitResponse, CHAT_DEBUG_TARGET, CHAT_EVENT_TARGET,
    MESSAGE_ENQUEUE_TARGET,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

const FIXTURES: &[(&str, &str)] = &[
    (
        "catalog.event.json",
        include_str!("../fixtures/catalog.event.json"),
    ),
    (
        "debug.event.json",
        include_str!("../fixtures/debug.event.json"),
    ),
    (
        "enqueue.request.json",
        include_str!("../fixtures/enqueue.request.json"),
    ),
    (
        "enqueue.response.json",
        include_str!("../fixtures/enqueue.response.json"),
    ),
    (
        "turn-submit.request.json",
        include_str!("../fixtures/turn-submit.request.json"),
    ),
    (
        "turn-submit.response.json",
        include_str!("../fixtures/turn-submit.response.json"),
    ),
    (
        "session-start.request.json",
        include_str!("../fixtures/session-start.request.json"),
    ),
    (
        "session-start.response.json",
        include_str!("../fixtures/session-start.response.json"),
    ),
    (
        "interaction-answer.request.json",
        include_str!("../fixtures/interaction-answer.request.json"),
    ),
    (
        "interaction-answer.response.json",
        include_str!("../fixtures/interaction-answer.response.json"),
    ),
    (
        "interaction.request.json",
        include_str!("../fixtures/interaction.request.json"),
    ),
    (
        "interaction.response.json",
        include_str!("../fixtures/interaction.response.json"),
    ),
    (
        "interaction.event.json",
        include_str!("../fixtures/interaction.event.json"),
    ),
    (
        "live.event.json",
        include_str!("../fixtures/live.event.json"),
    ),
    (
        "progress-thinking.event.json",
        include_str!("../fixtures/progress-thinking.event.json"),
    ),
    (
        "progress-planning.event.json",
        include_str!("../fixtures/progress-planning.event.json"),
    ),
    (
        "progress-waiting.event.json",
        include_str!("../fixtures/progress-waiting.event.json"),
    ),
    (
        "progress-tool-scheduled.event.json",
        include_str!("../fixtures/progress-tool-scheduled.event.json"),
    ),
    (
        "progress-tool-started.event.json",
        include_str!("../fixtures/progress-tool-started.event.json"),
    ),
    (
        "system-failed.event.json",
        include_str!("../fixtures/system-failed.event.json"),
    ),
    (
        "queue.event.json",
        include_str!("../fixtures/queue.event.json"),
    ),
    (
        "queue-reorder.request.json",
        include_str!("../fixtures/queue-reorder.request.json"),
    ),
    (
        "queue-reorder.response.json",
        include_str!("../fixtures/queue-reorder.response.json"),
    ),
    (
        "request-cancel.request.json",
        include_str!("../fixtures/request-cancel.request.json"),
    ),
    (
        "request-cancel.response.json",
        include_str!("../fixtures/request-cancel.response.json"),
    ),
    (
        "session.event.json",
        include_str!("../fixtures/session.event.json"),
    ),
    (
        "thread-archive.request.json",
        include_str!("../fixtures/thread-archive.request.json"),
    ),
    (
        "thread-archive.response.json",
        include_str!("../fixtures/thread-archive.response.json"),
    ),
    (
        "thread-create.request.json",
        include_str!("../fixtures/thread-create.request.json"),
    ),
    (
        "thread-create.response.json",
        include_str!("../fixtures/thread-create.response.json"),
    ),
    (
        "thread-list.request.json",
        include_str!("../fixtures/thread-list.request.json"),
    ),
    (
        "thread-list.response.json",
        include_str!("../fixtures/thread-list.response.json"),
    ),
    (
        "thread-load.request.json",
        include_str!("../fixtures/thread-load.request.json"),
    ),
    (
        "thread-load.response.json",
        include_str!("../fixtures/thread-load.response.json"),
    ),
    (
        "thread-rotate.request.json",
        include_str!("../fixtures/thread-rotate.request.json"),
    ),
    (
        "thread-rotate.response.json",
        include_str!("../fixtures/thread-rotate.response.json"),
    ),
];

fn fixture(name: &str) -> Value {
    let raw = FIXTURES
        .iter()
        .find_map(|(fixture_name, raw)| (*fixture_name == name).then_some(*raw))
        .unwrap_or_else(|| panic!("unknown fixture {name}"));
    serde_json::from_str(raw).unwrap()
}

fn body<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_value(fixture(name)["body"].clone()).unwrap()
}

#[test]
fn every_bundled_fixture_matches_the_bundled_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v2.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for (name, raw) in FIXTURES {
        let value: Value = serde_json::from_str(raw).unwrap();
        let errors = validator
            .iter_errors(&value)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(
            errors.is_empty(),
            "fixture {name} failed schema validation: {errors:?}"
        );
    }
}

#[test]
fn target_manifest_matches_the_rust_inventory() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/targets.v1.json")).unwrap();
    assert_eq!(manifest["contractVersion"], env!("CARGO_PKG_VERSION"));
    let request_targets = manifest["requestTargets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        request_targets,
        vec![
            conversation_api::THREAD_LIST_TARGET,
            conversation_api::THREAD_CREATE_TARGET,
            conversation_api::THREAD_ARCHIVE_TARGET,
            conversation_api::THREAD_ROTATE_TARGET,
            conversation_api::THREAD_LOAD_TARGET,
            conversation_api::MESSAGE_ENQUEUE_TARGET,
            conversation_api::TURN_SUBMIT_TARGET,
            conversation_api::SESSION_START_TARGET,
            conversation_api::INTERACTION_ANSWER_TARGET,
            conversation_api::QUEUE_REORDER_TARGET,
            conversation_api::REQUEST_CANCEL_TARGET,
            conversation_api::INTERACTION_SUBMIT_TARGET,
        ]
    );
    let event_targets = manifest["eventTargets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        event_targets,
        vec![
            conversation_api::CHAT_EVENT_TARGET,
            conversation_api::CHAT_DEBUG_TARGET,
        ]
    );
}

#[test]
fn request_and_response_fixtures_deserialize_to_their_typed_dtos() {
    body::<ThreadListRequest>("thread-list.request.json");
    body::<ThreadCreateRequest>("thread-create.request.json");
    body::<ThreadLoadRequest>("thread-load.request.json");
    body::<QueueReorderRequest>("queue-reorder.request.json");
    body::<RequestCancelRequest>("request-cancel.request.json");
    body::<InteractionSubmitRequest>("interaction.request.json");
    assert_eq!(
        fixture("enqueue.request.json")["target"],
        MESSAGE_ENQUEUE_TARGET
    );
    let enqueue = body::<MessageEnqueueRequest>("enqueue.request.json");
    assert_eq!(
        enqueue.access_mode,
        Some(ConversationAccessMode::Interactive)
    );
    assert_eq!(enqueue.temperature, Some(0.2));
    assert_eq!(enqueue.content.parts.len(), 2);
    assert!(matches!(
        enqueue.content.parts[1],
        MessagePart::Image { .. }
    ));
    body::<ThreadArchiveRequest>("thread-archive.request.json");
    body::<ThreadRotateRequest>("thread-rotate.request.json");
    body::<ThreadCatalog>("thread-list.response.json");
    body::<ThreadCatalog>("thread-create.response.json");
    body::<ThreadCatalog>("thread-archive.response.json");
    body::<ThreadCatalog>("thread-rotate.response.json");
    body::<ThreadLoadResponse>("thread-load.response.json");
    let enqueue_response = body::<MessageEnqueueResponse>("enqueue.response.json");
    assert_eq!(
        enqueue_response.disposition,
        MessageEnqueueDisposition::Queued
    );
    let turn = body::<TurnSubmitRequest>("turn-submit.request.json");
    assert_eq!(turn.occurred_at_unix_ms, 1_787_587_200_000);
    let turn_response = body::<TurnSubmitResponse>("turn-submit.response.json");
    assert_eq!(turn_response.disposition, TurnSubmitDisposition::Queued);
    assert_eq!(
        turn_response.session_disposition,
        TurnSessionDisposition::RotatedIdleTimeout
    );
    body::<SessionStartRequest>("session-start.request.json");
    let session_start = body::<SessionStartResponse>("session-start.response.json");
    assert_eq!(session_start.disposition, SessionStartDisposition::Rotated);
    body::<InteractionAnswerRequest>("interaction-answer.request.json");
    let interaction_answer = body::<InteractionAnswerResponse>("interaction-answer.response.json");
    assert_eq!(
        interaction_answer.disposition,
        InteractionAnswerDisposition::Accepted
    );
    let cancel_response = body::<RequestCancelResponse>("request-cancel.response.json");
    assert_eq!(cancel_response.phase, RequestCancelPhase::Running);
    assert_eq!(cancel_response.outcome, RequestCancelOutcome::Cancelling);
    body::<ConversationQueue>("queue-reorder.response.json");
    let interaction = body::<InteractionSubmitResponse>("interaction.response.json");
    assert_eq!(
        interaction.disposition,
        InteractionSubmitDisposition::Accepted
    );
}

#[test]
fn every_chat_event_deserializes_without_provider_knowledge() {
    for name in [
        "catalog.event.json",
        "interaction.event.json",
        "live.event.json",
        "progress-thinking.event.json",
        "progress-planning.event.json",
        "progress-waiting.event.json",
        "progress-tool-scheduled.event.json",
        "progress-tool-started.event.json",
        "system-failed.event.json",
        "queue.event.json",
        "session.event.json",
    ] {
        let event = fixture(name);
        assert_eq!(event["target"], CHAT_EVENT_TARGET);
        let typed: ConversationEvent = serde_json::from_value(event["body"].clone()).unwrap();
        assert!(!typed.surface().canonical_id().is_empty());
        assert!(!typed.event_type().is_empty());
    }

    let debug = fixture("debug.event.json");
    assert_eq!(debug["target"], CHAT_DEBUG_TARGET);
    body::<DebugEvent>("debug.event.json");
}

#[test]
fn missing_surface_identity_is_rejected() {
    let mut event = fixture("live.event.json")["body"].clone();
    event.as_object_mut().unwrap().remove("surfaceId");
    assert!(serde_json::from_value::<ConversationEvent>(event).is_err());
}

#[test]
fn conversation_events_reject_unknown_types_and_invalid_live_payloads() {
    let mut unknown = fixture("live.event.json")["body"].clone();
    unknown["type"] = Value::String("run.something_new".to_string());
    assert!(serde_json::from_value::<ConversationEvent>(unknown).is_err());

    let mut invalid_progress = fixture("progress-thinking.event.json")["body"].clone();
    invalid_progress["data"]["toolName"] = Value::String("search".to_string());
    assert!(serde_json::from_value::<ConversationEvent>(invalid_progress).is_err());

    let mut legacy_preview = fixture("live.event.json")["body"].clone();
    legacy_preview["data"]
        .as_object_mut()
        .unwrap()
        .remove("append");
    assert!(serde_json::from_value::<ConversationEvent>(legacy_preview).is_err());
}

#[test]
fn command_only_enums_reject_internal_state_values() {
    let mut rotate = fixture("thread-rotate.request.json");
    rotate["body"]["reason"] = Value::String("replaced".to_string());
    assert!(serde_json::from_value::<ThreadRotateRequest>(rotate["body"].clone()).is_err());

    let mut enqueue = fixture("enqueue.request.json");
    enqueue["body"]["accessMode"] = Value::String("internal".to_string());
    let schema: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v2.schema.json")).unwrap();
    assert!(!jsonschema::validator_for(&schema)
        .unwrap()
        .is_valid(&enqueue));
}
