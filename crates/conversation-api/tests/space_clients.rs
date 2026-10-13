use conversation_api::execution::{Actor, Scope};
use conversation_api::{ClientPrincipal, ConversationSurface};

fn scope() -> Scope {
    Scope {
        tenant_id: Some("tenant".into()),
        scope_id: "space".into(),
    }
}
fn actor() -> Actor {
    Actor {
        principal: ClientPrincipal::Space {
            tenant_id: "tenant".into(),
            scope_id: "space".into(),
        },
        client_id: Some("a:pod-one".into()),
        credential_id: Some("enrollment-one".into()),
    }
}

#[test]
fn group_is_independent_of_messaging_and_personal_surfaces() {
    let group = ConversationSurface::space_clients();
    assert!(group.is_group());
    assert!(group.is_client());
    assert_eq!(group.user_id(), None);
    assert_eq!(group.canonical_id(), "cs1:cg:c3BhY2UtY2xpZW50cw");
    assert_eq!(
        group.canonical_id().parse::<ConversationSurface>().unwrap(),
        group
    );
    assert_ne!(
        group,
        ConversationSurface::messaging_group("test", "account", "group", None).unwrap()
    );
    assert_ne!(
        group,
        ConversationSurface::client_personal("owner").unwrap()
    );
}

#[test]
fn a_space_actor_keeps_its_identity_and_requires_scope_bound_enrollment() {
    let mut actor = actor();
    assert_eq!(actor.user_id(), None);
    assert!(actor.valid_for(&scope(), &ConversationSurface::space_clients()));
    let mut other = scope();
    other.scope_id = "other".into();
    assert!(!actor.valid_for(&other, &ConversationSurface::space_clients()));
    other = scope();
    other.tenant_id = Some("other".into());
    assert!(!actor.valid_for(&other, &ConversationSurface::space_clients()));
    actor.credential_id = None;
    assert!(!actor.valid_for(&scope(), &ConversationSurface::space_clients()));
    actor = super_actor_without_client();
    assert!(!actor.valid_for(&scope(), &ConversationSurface::space_clients()));
}
fn super_actor_without_client() -> Actor {
    let mut value = actor();
    value.client_id = None;
    value
}

#[test]
fn selecting_a_surface_cannot_change_actor_authority() {
    let user = Actor::user("owner", Some("a:desktop".into()));
    assert!(!user.valid_for(&scope(), &ConversationSurface::space_clients()));
    assert!(!actor().valid_for(
        &scope(),
        &ConversationSurface::client_personal("owner").unwrap()
    ));
    assert!(!actor().valid_for(
        &scope(),
        &ConversationSurface::messaging_group("test", "bot", "group", None).unwrap()
    ));
    assert!(!user.valid_for(
        &scope(),
        &ConversationSurface::client_personal("another-user").unwrap()
    ));
}

#[test]
fn actor_wire_rejects_the_previous_user_only_shape_and_mixed_principals() {
    for wire in [
        serde_json::json!({"user_id":"owner","client_id":"a:pod"}),
        serde_json::json!({"principal":{"kind":"space","tenantId":"tenant","scopeId":"space","userId":"owner"},"client_id":"a:pod","credential_id":"enrollment"}),
    ] {
        assert!(serde_json::from_value::<Actor>(wire).is_err());
    }
    let value = actor();
    assert_eq!(
        serde_json::from_value::<Actor>(serde_json::to_value(&value).unwrap()).unwrap(),
        value
    );
}
