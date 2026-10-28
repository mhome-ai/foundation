use conversation_api::{
    InteractionAnswer, InteractionCard, InteractionKind, InteractionResponse,
    PendingInteractionItem,
};
use serde_json::{Value, json};

#[test]
fn public_and_execution_cards_have_the_same_lossless_contract() {
    let frame: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v2.schema.json")).unwrap();
    let execution: Value =
        serde_json::from_str(include_str!("../schema/execution/envelope.v1.schema.json")).unwrap();
    assert_eq!(
        frame["$defs"]["interactionCard"],
        execution["$defs"]["interactionCard"]
    );
    let validator = jsonschema::validator_for(&frame["$defs"]["interactionCard"]).unwrap();
    let card = json!({"type":"group.upsert","title":"Create group","message":"Review members","tone":"normal","continueLabel":"Create","cancelLabel":"Cancel","data":{"name":"Lights","members":[{"id":"d1","name":"Desk"}]}});
    assert!(validator.is_valid(&card));
    let typed: InteractionCard = serde_json::from_value(card.clone()).unwrap();
    assert!(typed.is_valid());
    assert_eq!(serde_json::to_value(typed).unwrap(), card);
    let mut legacy = card.clone();
    legacy["clientTask"] = json!({"type":"connect_integration"});
    assert!(!validator.is_valid(&legacy));
    assert!(serde_json::from_value::<InteractionCard>(legacy).is_err());
}

#[test]
fn responses_are_typed_and_bound_to_the_interaction_kind() {
    let mut pending = PendingInteractionItem {
        action_id: "a".into(),
        kind: InteractionKind::Approval,
        expires_at: None,
        card: InteractionCard::new("room.delete", "Delete room", "Review"),
    };
    let answer = |response| InteractionResponse {
        client_report: None,
        action_id: "a".into(),
        response,
    };
    assert!(pending.accepts(&answer(InteractionAnswer::Approve)));
    assert!(!pending.accepts(&answer(InteractionAnswer::Continue {
        data: json!({"roomId":"other"})
    })));
    pending.kind = InteractionKind::UserAction;
    assert!(pending.accepts(&answer(InteractionAnswer::Continue {
        data: json!({"name":"Final name"})
    })));
    assert!(pending.accepts(&answer(InteractionAnswer::Cancel)));
    assert!(!pending.accepts(&answer(InteractionAnswer::Approve)));
    assert!(!pending.accepts(&InteractionResponse {
        client_report: None,
        action_id: "stale".into(),
        response: InteractionAnswer::Cancel
    }));
    for invalid in [
        json!({"action":"approve","data":null}),
        json!({"action":"reject","target":"/app/arbitrary"}),
        json!({"action":"continue"}),
        json!({"action":"cancel","data":{}}),
    ] {
        assert!(
            serde_json::from_value::<InteractionAnswer>(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn facade_results_cannot_schedule_another_interaction() {
    let value = json!({"value":{"status":"incomplete"},"continuation":null});
    assert!(serde_json::from_value::<conversation_api::execution::FacadeResult>(value).is_err());
}

#[test]
fn client_reports_are_strict_and_only_belong_to_user_actions() {
    let report = json!({"warnings":[{"code":"SYNC_PENDING","message":"Saved; sync is pending"}]});
    let mut value = json!({"actionId":"a","response":{"action":"continue","data":{"spaceId":"s"}},"clientReport":report});
    let parsed: InteractionResponse = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(), value);
    let mut pending = PendingInteractionItem {
        action_id: "a".into(),
        kind: InteractionKind::UserAction,
        expires_at: None,
        card: InteractionCard::new("space.create", "Create", "Confirm"),
    };
    assert!(pending.accepts(&parsed));
    for path in [
        "../schema/conversation-frame.v2.schema.json",
        "../schema/execution/envelope.v1.schema.json",
    ] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join(path);
        let schema: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let validator =
            jsonschema::validator_for(&schema["$defs"]["interactionItemResponse"]).unwrap();
        assert!(validator.is_valid(&value));
        let mut invalid = value.clone();
        invalid["clientReport"]["warnings"][0]["details"] = json!({"token":"secret"});
        assert!(!validator.is_valid(&invalid));
        assert!(serde_json::from_value::<InteractionResponse>(invalid).is_err());
        let mut approval = value.clone();
        approval["response"] = json!({"action":"approve"});
        assert!(!validator.is_valid(&approval));
    }
    value["response"] = json!({"action":"cancel"});
    assert!(pending.accepts(&serde_json::from_value(value.clone()).unwrap()));
    pending.kind = InteractionKind::Approval;
    value["response"] = json!({"action":"approve"});
    assert!(!pending.accepts(&serde_json::from_value(value.clone()).unwrap()));
    pending.kind = InteractionKind::UserAction;
    value["response"] = json!({"action":"continue","data":null});
    for report in [
        json!({"warnings":[]}),
        json!({"warnings":[{"code":" ","message":"bad"}]}),
    ] {
        value["clientReport"] = report;
        assert!(!pending.accepts(&serde_json::from_value(value.clone()).unwrap()));
    }
}
