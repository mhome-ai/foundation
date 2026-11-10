use conversation_api::execution::{Actor, InvocationContext, SpaceContext, ThreadKey, stored};
use conversation_api::{ClientPrincipal, ConversationSurface};

fn context() -> InvocationContext {
    InvocationContext {
        space: SpaceContext {
            tenant_id: Some("tenant".into()),
            space_id: "home".into(),
        },
        actor: Actor {
            principal: ClientPrincipal::Space {
                tenant_id: "tenant".into(),
                space_id: "home".into(),
            },
            credential_id: Some("enrollment".into()),
            client_id: Some("a:pod".into()),
        },
        surface_id: ConversationSurface::space_clients(),
        thread_id: "thread".into(),
        run_id: "run".into(),
        operation_id: "op".into(),
        deadline_unix_ms: None,
        traceparent: None,
    }
}

#[test]
fn wire_uses_space_and_rejects_retired_identity_keys() {
    let expected = context();
    let wire = serde_json::to_value(&expected).unwrap();
    assert_eq!(wire["space"]["space_id"], "home");
    assert_eq!(wire["actor"]["principal"]["spaceId"], "home");
    assert!(wire.get("scope").is_none());
    assert_eq!(
        serde_json::from_value::<InvocationContext>(wire.clone()).unwrap(),
        expected
    );
    let mut old = wire.clone();
    old["scope"] = old.as_object_mut().unwrap().remove("space").unwrap();
    assert!(serde_json::from_value::<InvocationContext>(old).is_err());
    let mut old = wire;
    old["space"]["scope_id"] = old["space"]
        .as_object_mut()
        .unwrap()
        .remove("space_id")
        .unwrap();
    assert!(serde_json::from_value::<InvocationContext>(old).is_err());
    assert!(
        serde_json::from_value::<ClientPrincipal>(
            serde_json::json!({"kind":"space","tenantId":"tenant","scopeId":"home"})
        )
        .is_err()
    );
}

#[test]
fn private_context_and_thread_identity_keep_the_existing_storage_shape() {
    let expected = context();
    let encoded = stored::context::to_string(&expected).unwrap();
    let value: serde_json::Value = serde_json::from_str(&encoded).unwrap();
    assert_eq!(
        value["scope"],
        serde_json::json!({"tenant_id":"tenant","scope_id":"home"})
    );
    assert_eq!(
        value["actor"]["principal"],
        serde_json::json!({"kind":"space","tenantId":"tenant","scopeId":"home"})
    );
    assert!(value.get("space").is_none());
    assert_eq!(stored::context::from_str(&encoded).unwrap(), expected);
    assert!(serde_json::from_str::<InvocationContext>(&encoded).is_err());
    let key = ThreadKey {
        scope: expected.space,
        surface_id: expected.surface_id,
        thread_id: expected.thread_id,
    };
    let bytes = serde_json::to_string(&key).unwrap();
    let value: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    assert_eq!(value["scope"]["scope_id"], "home");
    assert_eq!(serde_json::from_str::<ThreadKey>(&bytes).unwrap(), key);
}
