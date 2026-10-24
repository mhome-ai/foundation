use core_api::messaging::{IdentityRedemptionContext, IngressRedeem, MessagingIngressOperation};
use serde_json::{json, Value};

fn fixture() -> Value {
    serde_json::from_str(include_str!("../fixtures/messaging-redemption.json")).unwrap()
}

#[test]
fn redemption_wire_round_trips_without_message_content() {
    for kind in ["link", "bind"] {
        let mut value = fixture();
        value["kind"] = json!(kind);
        let request: IngressRedeem = serde_json::from_value(value.clone()).unwrap();
        request.context.clone().validate().unwrap();
        let operation = MessagingIngressOperation::Redeem(request);
        assert_eq!(operation.target(), "/messaging/ingress/redeem");
        assert_eq!(
            serde_json::from_str::<Value>(&operation.payload().unwrap()).unwrap(),
            value
        );
        let envelope = serde_json::to_value(&operation).unwrap();
        assert_eq!(envelope, json!({"operation": "redeem", "input": value}));
        let decoded: MessagingIngressOperation = serde_json::from_value(envelope).unwrap();
        assert_eq!(decoded.payload().unwrap(), operation.payload().unwrap());
    }
}

#[test]
fn redemption_rejects_old_message_envelopes_and_content_fields() {
    let mut legacy = fixture();
    let context = legacy.as_object_mut().unwrap().remove("context").unwrap();
    legacy["event"] = context;
    assert!(serde_json::from_value::<IngressRedeem>(legacy).is_err());
    for field in ["content", "schemaVersion", "occurredAtMs"] {
        let mut value = fixture();
        value["context"][field] = json!(null);
        assert!(
            serde_json::from_value::<IngressRedeem>(value).is_err(),
            "{field}"
        );
    }
}

#[test]
fn context_preserves_identity_checks_and_normalizes_only_the_conversation_lane() {
    let mut value = fixture()["context"].clone();
    value["address"]["laneId"] = json!("topic");
    let context: IdentityRedemptionContext = serde_json::from_value(value.clone()).unwrap();
    let validated = context.validate().unwrap();
    assert!(validated.address.lane_id.is_none());
    assert_eq!(validated.event_id, "provider-event");
    assert_eq!(validated.actor.external_user_id, "sender");
    for (field, replacement) in [("provider", "feishu"), ("accountId", "other-bot")] {
        let mut invalid = value.clone();
        invalid["actor"][field] = json!(replacement);
        assert!(serde_json::from_value::<IdentityRedemptionContext>(invalid)
            .unwrap()
            .validate()
            .is_err());
    }
    for field in ["eventId", "conversationDisplayName"] {
        for invalid_text in ["", " ", " padded "] {
            let mut invalid = value.clone();
            invalid[field] = json!(invalid_text);
            assert!(serde_json::from_value::<IdentityRedemptionContext>(invalid)
                .unwrap()
                .validate()
                .is_err());
        }
    }
}
