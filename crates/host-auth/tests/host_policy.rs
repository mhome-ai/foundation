use core_api::{
    runtime_space::{RuntimeSpacePolicy, SpaceIdentity},
    RuntimeEnv,
};
use host_auth::policy::{AccessBinding, Denied, HostMode, HostPolicy, Permission};
use std::collections::BTreeSet;

fn space() -> SpaceIdentity {
    SpaceIdentity {
        tenant_id: "tenant-a".into(),
        scope_id: "space-a".into(),
    }
}
fn unclaimed(mode: HostMode) -> HostPolicy {
    HostPolicy::unclaimed(
        "host-a".into(),
        RuntimeEnv::Dev,
        mode,
        "generation-a".into(),
    )
    .unwrap()
}
fn binding(user: &str, space: Option<SpaceIdentity>) -> AccessBinding {
    AccessBinding {
        host_id: "host-a".into(),
        host_key_id: "key-a".into(),
        env: RuntimeEnv::Dev,
        generation: "generation-a".into(),
        user_id: user.into(),
        space,
    }
}
fn permissions() -> BTreeSet<Permission> {
    BTreeSet::from([
        Permission::Read,
        Permission::ManageRuntime,
        Permission::ManageAdministrators,
        Permission::Reset,
    ])
}

#[test]
fn admin_mode_requires_a_current_administrator_and_no_space_authority() {
    let mut host = unclaimed(HostMode::Admin);
    host.claim_verified("alice", None).unwrap();
    for action in permissions() {
        assert!(host
            .authorize(&binding("alice", None), &permissions(), action)
            .is_ok());
        assert_eq!(
            host.authorize(&binding("bob", None), &permissions(), action),
            Err(Denied::NotAdministrator)
        );
        assert_eq!(
            host.authorize(&binding("alice", Some(space())), &permissions(), action),
            Err(Denied::PrincipalMismatch)
        );
    }
    host.add_verified_administrator("bob".into()).unwrap();
    host.remove_verified_administrator("alice").unwrap();
    assert_eq!(
        host.authorize(&binding("alice", None), &permissions(), Permission::Read),
        Err(Denied::NotAdministrator)
    );
    assert_eq!(
        host.remove_verified_administrator("bob"),
        Err(Denied::LastAdministrator)
    );
}

#[test]
fn space_mode_uses_verified_current_owner_without_persisting_that_user_as_owner() {
    let mut host = unclaimed(HostMode::SpaceSelf);
    host.claim_verified("alice", Some(&space())).unwrap();
    for user in ["alice", "new-owner"] {
        for action in [
            Permission::Read,
            Permission::ManageRuntime,
            Permission::Reset,
        ] {
            assert!(host
                .authorize(&binding(user, Some(space())), &permissions(), action)
                .is_ok());
        }
    }
    assert_eq!(
        host.authorize(&binding("alice", None), &permissions(), Permission::Read),
        Err(Denied::PrincipalMismatch)
    );
    assert_eq!(
        host.authorize(
            &binding("alice", Some(space())),
            &permissions(),
            Permission::ManageAdministrators
        ),
        Err(Denied::PrincipalMismatch)
    );
    let other = SpaceIdentity {
        scope_id: "space-b".into(),
        ..space()
    };
    assert_eq!(
        host.authorize(
            &binding("alice", Some(other)),
            &permissions(),
            Permission::Read
        ),
        Err(Denied::PrincipalMismatch)
    );
}

#[test]
fn identity_environment_generation_and_action_are_checked_even_for_an_owner() {
    let mut host = unclaimed(HostMode::SpaceSelf);
    host.claim_verified("alice", Some(&space())).unwrap();
    let original = binding("alice", Some(space()));
    let variants = [
        AccessBinding {
            host_id: "other".into(),
            ..original.clone()
        },
        AccessBinding {
            generation: "old-generation".into(),
            ..original.clone()
        },
        AccessBinding {
            env: RuntimeEnv::Prod,
            ..original.clone()
        },
        AccessBinding {
            user_id: String::new(),
            ..original.clone()
        },
    ];
    for invalid in variants {
        assert_eq!(
            host.authorize(&invalid, &permissions(), Permission::Read),
            Err(Denied::BindingMismatch)
        );
    }
    assert_eq!(
        host.authorize(
            &original,
            &BTreeSet::from([Permission::Read]),
            Permission::ManageRuntime
        ),
        Err(Denied::PermissionMissing)
    );
}

#[test]
fn mode_and_space_binding_cannot_change_without_reset() {
    let mut host = unclaimed(HostMode::SpaceSelf);
    assert!(host.runtime_policy().unwrap().is_none());
    assert!(host.claim_verified("alice", None).is_err());
    host.claim_verified("alice", Some(&space())).unwrap();
    assert_eq!(
        host.runtime_policy().unwrap(),
        Some(RuntimeSpacePolicy::Single { space: space() })
    );
    // A fresh attempt confirms the immutable Space, including after ownership
    // changes. Cloud verifies current ownership before issuing the proof.
    host.claim_verified("alice", Some(&space())).unwrap();
    host.claim_verified("bob", Some(&space())).unwrap();
    let other = SpaceIdentity {
        scope_id: "other".into(),
        ..space()
    };
    assert_eq!(
        host.claim_verified("alice", Some(&other)),
        Err(Denied::AlreadyClaimed)
    );
    assert_eq!(host.validate(HostMode::Admin), Err(Denied::ModeChanged));
    assert_eq!(
        unclaimed(HostMode::Admin).runtime_policy().unwrap(),
        Some(RuntimeSpacePolicy::Multi {})
    );
}

#[test]
fn fresh_admin_confirmation_cannot_add_or_restore_an_administrator() {
    let mut host = unclaimed(HostMode::Admin);
    host.claim_verified("alice", None).unwrap();
    let original = host.clone();
    host.claim_verified("alice", None).unwrap();
    assert_eq!(host, original);
    assert_eq!(
        host.claim_verified("bob", None),
        Err(Denied::AlreadyClaimed)
    );
    host.add_verified_administrator("bob".into()).unwrap();
    host.remove_verified_administrator("alice").unwrap();
    assert_eq!(
        host.claim_verified("alice", None),
        Err(Denied::AlreadyClaimed)
    );
}

#[test]
fn tokens_cannot_represent_space_clients_or_core_as_a_management_principal() {
    let mut value = serde_json::to_value(binding("alice", Some(space()))).unwrap();
    value["controllerKind"] = serde_json::json!("spaceClient");
    assert!(serde_json::from_value::<AccessBinding>(value).is_err());
    let mut value = serde_json::to_value(binding("alice", None)).unwrap();
    value["principal"] = serde_json::json!({"kind":"core","hubId":"hub-a"});
    assert!(serde_json::from_value::<AccessBinding>(value).is_err());
}
