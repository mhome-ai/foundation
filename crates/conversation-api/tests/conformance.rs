use conversation_api::{
    ConversationEvent, MessageEnqueueRequest, ThreadCatalog, CHAT_EVENT_TARGET,
    MESSAGE_ENQUEUE_TARGET,
};
use serde_json::Value;

fn fixture(name: &str) -> Value {
    let raw = match name {
        "enqueue.request.json" => include_str!("../fixtures/enqueue.request.json"),
        "thread-list.response.json" => include_str!("../fixtures/thread-list.response.json"),
        "interaction.event.json" => include_str!("../fixtures/interaction.event.json"),
        _ => panic!("unknown fixture"),
    };
    serde_json::from_str(raw).unwrap()
}

#[test]
fn canonical_fixtures_match_the_bundled_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    for name in [
        "enqueue.request.json",
        "thread-list.response.json",
        "interaction.event.json",
    ] {
        let value = fixture(name);
        assert!(
            validator.is_valid(&value),
            "fixture {name} failed schema validation"
        );
    }
}

#[test]
fn request_and_response_bodies_deserialize_to_typed_dtos() {
    let enqueue = fixture("enqueue.request.json");
    assert_eq!(enqueue["target"], MESSAGE_ENQUEUE_TARGET);
    serde_json::from_value::<MessageEnqueueRequest>(enqueue["body"].clone()).unwrap();

    let catalog = fixture("thread-list.response.json");
    serde_json::from_value::<ThreadCatalog>(catalog["body"].clone()).unwrap();
}

#[test]
fn chat_events_deserialize_without_provider_knowledge() {
    let event = fixture("interaction.event.json");
    assert_eq!(event["target"], CHAT_EVENT_TARGET);
    serde_json::from_value::<ConversationEvent>(event["body"].clone()).unwrap();
}
