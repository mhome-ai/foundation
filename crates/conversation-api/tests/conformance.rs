use conversation_api::{
    ConversationEvent, ConversationQueue, DebugEvent, InteractionSubmitRequest,
    InteractionSubmitResponse, MessageEnqueueRequest, MessageEnqueueResponse, QueueReorderRequest,
    RequestCancelRequest, RequestCancelResponse, ThreadArchiveRequest, ThreadCatalog,
    ThreadCreateRequest, ThreadListRequest, ThreadLoadRequest, ThreadLoadResponse,
    ThreadRotateRequest, CHAT_DEBUG_TARGET, CHAT_EVENT_TARGET, MESSAGE_ENQUEUE_TARGET,
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
        serde_json::from_str(include_str!("../schema/conversation-frame.v1.schema.json")).unwrap();
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
    assert_eq!(enqueue.access_mode.as_deref(), Some("interactive"));
    assert_eq!(enqueue.temperature, Some(0.2));
    assert_eq!(enqueue.image_refs.len(), 1);
    body::<ThreadArchiveRequest>("thread-archive.request.json");
    body::<ThreadRotateRequest>("thread-rotate.request.json");
    body::<ThreadCatalog>("thread-list.response.json");
    body::<ThreadCatalog>("thread-create.response.json");
    body::<ThreadCatalog>("thread-archive.response.json");
    body::<ThreadCatalog>("thread-rotate.response.json");
    body::<ThreadLoadResponse>("thread-load.response.json");
    body::<MessageEnqueueResponse>("enqueue.response.json");
    body::<RequestCancelResponse>("request-cancel.response.json");
    body::<ConversationQueue>("queue-reorder.response.json");
    body::<InteractionSubmitResponse>("interaction.response.json");
}

#[test]
fn every_chat_event_deserializes_without_provider_knowledge() {
    for name in [
        "catalog.event.json",
        "interaction.event.json",
        "live.event.json",
        "queue.event.json",
        "session.event.json",
    ] {
        let event = fixture(name);
        assert_eq!(event["target"], CHAT_EVENT_TARGET);
        let typed: ConversationEvent = serde_json::from_value(event["body"].clone()).unwrap();
        assert!(!typed.surface_id().is_empty());
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
fn command_only_enums_reject_internal_state_values() {
    let mut rotate = fixture("thread-rotate.request.json");
    rotate["body"]["reason"] = Value::String("replaced".to_string());
    assert!(serde_json::from_value::<ThreadRotateRequest>(rotate["body"].clone()).is_err());

    let mut enqueue = fixture("enqueue.request.json");
    enqueue["body"]["accessMode"] = Value::String("internal".to_string());
    let schema: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v1.schema.json")).unwrap();
    assert!(!jsonschema::validator_for(&schema)
        .unwrap()
        .is_valid(&enqueue));
}
