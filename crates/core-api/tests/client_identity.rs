use core_api::client_identity::{AuthenticatedClient, ClientIdentityError, ClientPrincipal};
use serde_json::json;

fn space_client() -> AuthenticatedClient {
    serde_json::from_value(json!({
        "appClientId": "a:device-1", "credentialId": "enrollment-2",
        "principal": {"kind": "space", "tenantId": "tenant", "scopeId": "space"}
    }))
    .unwrap()
}

#[test]
fn space_client_has_no_acting_user_and_cannot_switch_target() {
    let client = space_client();
    assert_eq!(client.user_id(), None);
    assert!(client.is_space_client());
    assert!(client.validate_for("tenant", "space").is_ok());
    assert_eq!(
        client.validate_for("tenant", "other"),
        Err(ClientIdentityError::SpaceMismatch)
    );
    assert_eq!(
        client.validate_for("other", "space"),
        Err(ClientIdentityError::SpaceMismatch)
    );
}

#[test]
fn personal_pod_keeps_user_authority_without_a_separate_pod_principal() {
    let client = AuthenticatedClient {
        app_client_id: "a:device-1".into(),
        credential_id: Some("enrollment-2".into()),
        principal: ClientPrincipal::User {
            user_id: "user".into(),
        },
    };
    assert_eq!(client.user_id(), Some("user"));
    assert!(!client.is_space_client());
    // Membership authorization is separate; identity does not pin a user to
    // the Space selected during Pod setup.
    assert!(client.validate_for("tenant", "space").is_ok());
    assert!(client.validate_for("tenant", "another-space").is_ok());
}

#[test]
fn missing_kind_or_legacy_user_field_never_selects_a_user_fallback() {
    for principal in [
        json!({"userId":"user"}),
        json!({"kind":"space","tenantId":"tenant","scopeId":"space","userId":"owner"}),
        json!({"kind":"core","tenantId":"tenant","scopeId":"space"}),
    ] {
        assert!(serde_json::from_value::<AuthenticatedClient>(json!({
            "appClientId":"a:device-1", "credentialId":"enrollment", "principal":principal
        }))
        .is_err());
    }
}

#[test]
fn a_space_client_requires_an_enrollment_and_canonical_app_client_id() {
    let mut client = space_client();
    client.credential_id = None;
    assert_eq!(
        client.validate_for("tenant", "space"),
        Err(ClientIdentityError::MissingEnrollment)
    );
    client.credential_id = Some("enrollment".into());
    for invalid in [
        "",
        "C:peer:socket",
        "a:",
        "a:a:device",
        "a:p:device",
        "a:device\n",
    ] {
        client.app_client_id = invalid.into();
        assert_eq!(
            client.validate_for("tenant", "space"),
            Err(ClientIdentityError::InvalidIdentifier)
        );
    }
}

#[test]
fn re_enrollment_changes_credential_without_changing_installation_identity() {
    let previous = space_client();
    let mut current = previous.clone();
    current.credential_id = Some("new-enrollment".into());
    assert_eq!(previous.app_client_id, current.app_client_id);
    assert_ne!(previous, current);
    assert_eq!(
        serde_json::to_value(&current).unwrap()["credentialId"],
        "new-enrollment"
    );
}

#[test]
fn empty_user_or_credential_never_becomes_an_anonymous_or_internal_actor() {
    let mut client = space_client();
    client.principal = ClientPrincipal::User {
        user_id: String::new(),
    };
    assert_eq!(
        client.validate_for("tenant", "space"),
        Err(ClientIdentityError::InvalidIdentifier)
    );
    client.principal = ClientPrincipal::User {
        user_id: "user".into(),
    };
    client.credential_id = Some(" ".into());
    assert_eq!(
        client.validate_for("tenant", "space"),
        Err(ClientIdentityError::InvalidIdentifier)
    );
    client.credential_id = None;
    assert!(client.validate_for("tenant", "space").is_ok());
    assert_eq!(
        client.validate_for("", "space"),
        Err(ClientIdentityError::InvalidIdentifier)
    );
}

#[test]
fn space_catalog_survives_new_devices_and_re_enrollment() {
    let first = space_client();
    let mut second = first.clone();
    second.app_client_id = "a:device-2".into();
    second.credential_id = Some("another-enrollment".into());
    assert_eq!(
        first.conversation_surface("tenant", "space"),
        second.conversation_surface("tenant", "space")
    );
    second.credential_id = Some("after-factory-reset".into());
    assert_eq!(
        first.conversation_surface("tenant", "space"),
        second.conversation_surface("tenant", "space")
    );
    second.credential_id = None;
    assert_eq!(
        second.conversation_surface("tenant", "space"),
        Err(ClientIdentityError::MissingEnrollment)
    );
    assert_eq!(
        first.conversation_surface("tenant", "other"),
        Err(ClientIdentityError::SpaceMismatch)
    );
}

#[test]
fn personal_clients_share_user_catalog_without_joining_space_group() {
    let mut client = space_client();
    let group = client.conversation_surface("tenant", "space").unwrap();
    client.principal = ClientPrincipal::User {
        user_id: "alice".into(),
    };
    let personal = client.conversation_surface("tenant", "space").unwrap();
    client.app_client_id = "a:another-device".into();
    assert_eq!(
        personal,
        client.conversation_surface("tenant", "space").unwrap()
    );
    assert_ne!(personal, group);
    client.principal = ClientPrincipal::User {
        user_id: "bob".into(),
    };
    assert_ne!(
        personal,
        client.conversation_surface("tenant", "space").unwrap()
    );
}
