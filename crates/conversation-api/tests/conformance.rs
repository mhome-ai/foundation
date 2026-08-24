use conversation_api::{
    ConversationEvent, DebugEvent, MessageEnqueueRequest, RequestCancelResponse,
    ThreadArchiveRequest, ThreadCatalog, ThreadLoadResponse, ThreadRotateRequest,
    CHAT_DEBUG_TARGET, CHAT_EVENT_TARGET, MESSAGE_ENQUEUE_TARGET,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

const FIXTURES: &[(&str, &str)] = &[
    (
        "debug.event.json",
        include_str!("../fixtures/debug.event.json"),
    ),
    (
        "enqueue.request.json",
        include_str!("../fixtures/enqueue.request.json"),
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
        "queue-reorder.response.json",
        include_str!("../fixtures/queue-reorder.response.json"),
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
        "thread-list.response.json",
        include_str!("../fixtures/thread-list.response.json"),
    ),
    (
        "thread-load.response.json",
        include_str!("../fixtures/thread-load.response.json"),
    ),
    (
        "thread-rotate.request.json",
        include_str!("../fixtures/thread-rotate.request.json"),
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
    assert_eq!(
        fixture("enqueue.request.json")["target"],
        MESSAGE_ENQUEUE_TARGET
    );
    body::<MessageEnqueueRequest>("enqueue.request.json");
    body::<ThreadArchiveRequest>("thread-archive.request.json");
    body::<ThreadRotateRequest>("thread-rotate.request.json");
    body::<ThreadCatalog>("thread-list.response.json");
    body::<ThreadLoadResponse>("thread-load.response.json");
    body::<RequestCancelResponse>("request-cancel.response.json");
    body::<conversation_api::ConversationQueue>("queue-reorder.response.json");
}

#[test]
fn every_chat_event_deserializes_without_provider_knowledge() {
    for name in [
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
