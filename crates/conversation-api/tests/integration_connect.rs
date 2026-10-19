use conversation_api::{ClientTask, InteractionPreview};
use serde_json::{Value, json};

#[test]
fn connection_task_is_session_bound_and_identical_in_both_wire_contracts() {
    let frame: Value =
        serde_json::from_str(include_str!("../schema/conversation-frame.v2.schema.json")).unwrap();
    let execution: Value =
        serde_json::from_str(include_str!("../schema/execution/envelope.v1.schema.json")).unwrap();
    let task = &frame["$defs"]["interactionPreview"]["properties"]["clientTask"];
    assert_eq!(
        task,
        &execution["$defs"]["interaction_preview"]["properties"]["clientTask"]
    );
    let mut schema = task.clone();
    schema["$defs"] = frame["$defs"].clone();
    let validator = jsonschema::validator_for(&schema).unwrap();

    let valid = json!({"type":"connect_integration", "integrationId":"provider.example", "sessionId":"session"});
    assert!(validator.is_valid(&valid));
    assert_eq!(
        serde_json::from_value::<ClientTask>(valid.clone()).unwrap(),
        ClientTask::ConnectIntegration {
            integration_id: "provider.example".into(),
            session_id: "session".into()
        }
    );
    for invalid in [
        json!({"type":"connect_integration", "integrationId":"a"}),
        json!({"type":"connect_integration", "integrationId":"a", "sessionId":12}),
        json!({"type":"install_integrations", "integrationIds":["provider.example"]}),
        json!({"type":"install_integration", "integrationIds":["a", "b"]}),
        json!({"type":"install_integration", "integrationId":["a"]}),
        json!({"type":"install_integration"}),
        json!({"type":"install_integration", "integrationId":"a", "integrationIds":["a"]}),
    ] {
        assert!(!validator.is_valid(&invalid), "{invalid}");
        assert!(serde_json::from_value::<ClientTask>(invalid).is_err());
    }
    for field in ["integrationId", "sessionId"] {
        for id in ["", "  "] {
            let mut invalid = valid.clone();
            invalid[field] = json!(id);
            assert!(!validator.is_valid(&invalid));
            let mut preview = InteractionPreview::confirmation("install", "Install", "Review");
            preview.client_task = Some(serde_json::from_value(invalid).unwrap());
            assert!(!preview.is_valid());
        }
    }
}
