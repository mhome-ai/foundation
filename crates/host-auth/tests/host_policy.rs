use core_api::RuntimeEnv;
use host_auth::{
    policy::{Denied, HostMode, HostPolicy, Permission},
    token_session::{AuthorizationLease, Binding, ControllerKind, Principal, Session},
};
use std::collections::BTreeSet;

fn user() -> Principal {
    Principal::User {
        user_id: "alice".into(),
    }
}

fn space() -> Principal {
    Principal::Space {
        tenant_id: "tenant-a".into(),
        space_id: "home-a".into(),
    }
}

fn binding(principal: Principal, controller_kind: ControllerKind) -> Binding {
    Binding {
        host_id: "host-a".into(),
        env: RuntimeEnv::Dev,
        binding_generation: "generation-a".into(),
        controller_id: "controller-a".into(),
        controller_kind,
        principal,
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

fn claimed(mode: HostMode) -> HostPolicy {
    let mut host = unclaimed(mode);
    host.claim_verified(&match mode {
        HostMode::Admin => user(),
        HostMode::Space => space(),
    })
    .unwrap();
    host
}

fn all_permissions() -> BTreeSet<Permission> {
    BTreeSet::from([
        Permission::Read,
        Permission::ManageRuntime,
        Permission::ManageAdministrators,
        Permission::Reset,
    ])
}

#[test]
fn management_matrix_distinguishes_clients_from_core_even_with_all_permissions() {
    use ControllerKind::{Core, SpaceClient, UserClient};
    use Denied::{PrincipalMismatch, ReadOnlyClient};
    use HostMode::{Admin, Space};
    use Permission::{ManageAdministrators, ManageRuntime, Read, Reset};

    let cases = [
        (Admin, user(), UserClient, Read, Ok(())),
        (Admin, user(), UserClient, ManageRuntime, Ok(())),
        (Admin, user(), UserClient, ManageAdministrators, Ok(())),
        (Admin, user(), UserClient, Reset, Ok(())),
        (Space, user(), UserClient, Read, Err(PrincipalMismatch)),
        (
            Space,
            user(),
            UserClient,
            ManageRuntime,
            Err(PrincipalMismatch),
        ),
        (Admin, space(), SpaceClient, Read, Err(PrincipalMismatch)),
        (
            Admin,
            space(),
            SpaceClient,
            ManageRuntime,
            Err(PrincipalMismatch),
        ),
        (Space, space(), SpaceClient, Read, Ok(())),
        (
            Space,
            space(),
            SpaceClient,
            ManageRuntime,
            Err(ReadOnlyClient),
        ),
        (
            Space,
            space(),
            SpaceClient,
            ManageAdministrators,
            Err(ReadOnlyClient),
        ),
        (Space, space(), SpaceClient, Reset, Err(ReadOnlyClient)),
        (Admin, user(), Core, ManageRuntime, Ok(())),
        (Admin, space(), Core, ManageRuntime, Err(PrincipalMismatch)),
        (Space, space(), Core, Read, Ok(())),
        (Space, space(), Core, ManageRuntime, Ok(())),
        (Space, space(), Core, Reset, Ok(())),
        (
            Space,
            space(),
            Core,
            ManageAdministrators,
            Err(PrincipalMismatch),
        ),
        (Space, user(), Core, ManageRuntime, Err(PrincipalMismatch)),
    ];
    for (mode, principal, kind, action, expected) in cases {
        let caller = binding(principal, kind);
        assert_eq!(
            claimed(mode).authorize(&caller, &all_permissions(), action),
            expected,
            "{mode:?}: {caller:?} -> {action:?}"
        );
    }
}

#[test]
fn personal_pod_and_other_user_clients_share_admin_policy() {
    let host = claimed(HostMode::Admin);
    for id in ["desktop", "mobile", "personal-pod"] {
        let mut caller = binding(user(), ControllerKind::UserClient);
        caller.controller_id = id.into();
        assert_eq!(
            host.authorize(&caller, &all_permissions(), Permission::ManageRuntime),
            Ok(())
        );
    }
}

#[test]
fn permissions_can_narrow_policy_but_never_widen_it() {
    let caller = binding(user(), ControllerKind::UserClient);
    let host = claimed(HostMode::Admin);
    assert_eq!(
        host.authorize(&caller, &BTreeSet::new(), Permission::Read),
        Err(Denied::PermissionMissing)
    );
    assert_eq!(
        host.authorize(
            &caller,
            &BTreeSet::from([Permission::Read]),
            Permission::ManageRuntime
        ),
        Err(Denied::PermissionMissing)
    );
    let stranger = binding(
        Principal::User {
            user_id: "bob".into(),
        },
        ControllerKind::UserClient,
    );
    assert_eq!(
        host.authorize(&stranger, &all_permissions(), Permission::Read),
        Err(Denied::NotAdministrator)
    );
}

#[test]
fn space_authority_cannot_cross_tenants_or_spaces() {
    let host = claimed(HostMode::Space);
    for kind in [ControllerKind::SpaceClient, ControllerKind::Core] {
        for (tenant_id, space_id) in [("tenant-b", "home-a"), ("tenant-a", "home-b")] {
            let caller = binding(
                Principal::Space {
                    tenant_id: tenant_id.into(),
                    space_id: space_id.into(),
                },
                kind,
            );
            assert_eq!(
                host.authorize(&caller, &all_permissions(), Permission::Read),
                Err(Denied::PrincipalMismatch)
            );
        }
    }
}

#[test]
fn host_environment_and_reset_generation_are_authorization_boundaries() {
    let host = claimed(HostMode::Admin);
    let caller = binding(user(), ControllerKind::UserClient);
    let mut another_host = caller.clone();
    another_host.host_id = "host-b".into();
    let mut another_env = caller.clone();
    another_env.env = RuntimeEnv::Prod;
    let mut old_generation = caller.clone();
    old_generation.binding_generation = "before-reset".into();
    let mut malformed = caller;
    malformed.controller_id = " controller-a".into();
    for caller in [another_host, another_env, old_generation, malformed] {
        assert_eq!(
            host.authorize(&caller, &all_permissions(), Permission::Read),
            Err(Denied::BindingMismatch)
        );
    }
}

#[test]
fn unclaimed_host_requires_first_claim_before_authenticated_operations() {
    for (mode, principal, kind) in [
        (HostMode::Admin, user(), ControllerKind::UserClient),
        (HostMode::Space, space(), ControllerKind::Core),
    ] {
        assert_eq!(
            unclaimed(mode).authorize(
                &binding(principal, kind),
                &all_permissions(),
                Permission::Read
            ),
            Err(Denied::Unclaimed)
        );
    }
}

#[test]
fn first_claim_is_mode_bound_idempotent_and_cannot_add_another_owner() {
    for (mode, initial, wrong_mode, second) in [
        (
            HostMode::Admin,
            user(),
            space(),
            Principal::User {
                user_id: "bob".into(),
            },
        ),
        (
            HostMode::Space,
            space(),
            user(),
            Principal::Space {
                tenant_id: "tenant-a".into(),
                space_id: "home-b".into(),
            },
        ),
    ] {
        let mut host = unclaimed(mode);
        assert_eq!(
            host.claim_verified(&wrong_mode),
            Err(Denied::PrincipalMismatch)
        );
        assert!(host.claim().is_none());
        host.claim_verified(&initial).unwrap();
        let committed = host.clone();
        assert_eq!(host.claim_verified(&initial), Ok(()));
        assert_eq!(host.claim_verified(&second), Err(Denied::AlreadyClaimed));
        assert_eq!(host, committed);
    }
}

#[test]
fn removed_admin_loses_access_despite_unexpired_session_and_last_admin_cannot_leave() {
    const NOW: u64 = 1_800_000_000;
    let mut host = claimed(HostMode::Admin);
    let caller = binding(user(), ControllerKind::UserClient);
    let lease = AuthorizationLease::from_verified_authority(NOW, NOW + 86_400, NOW).unwrap();
    let (session, access) =
        Session::establish(caller.clone(), "session-a".into(), lease, NOW).unwrap();
    assert_eq!(
        host.remove_verified_administrator("alice"),
        Err(Denied::LastAdministrator)
    );
    host.add_verified_administrator("bob".into()).unwrap();
    host.remove_verified_administrator("alice").unwrap();
    assert_eq!(session.authorize(&caller, &access, NOW + 1), Ok(()));
    assert_eq!(
        host.authorize(&caller, &all_permissions(), Permission::ManageRuntime),
        Err(Denied::NotAdministrator)
    );
    assert_eq!(host.claim_verified(&user()), Err(Denied::AlreadyClaimed));
    assert_eq!(
        host.remove_verified_administrator("bob"),
        Err(Denied::LastAdministrator)
    );
    assert_eq!(host.generation(), "generation-a");
}

#[test]
fn administrator_enrollment_cannot_bypass_first_claim_or_space_mode() {
    let mut host = unclaimed(HostMode::Admin);
    assert_eq!(
        host.add_verified_administrator("alice".into()),
        Err(Denied::Unclaimed)
    );
    let mut host = claimed(HostMode::Space);
    assert_eq!(
        host.add_verified_administrator("alice".into()),
        Err(Denied::PrincipalMismatch)
    );
    assert_eq!(
        host.remove_verified_administrator("alice"),
        Err(Denied::PrincipalMismatch)
    );
}

#[test]
fn restored_mode_and_claim_are_validated_without_reinterpreting_ownership() {
    for mode in [HostMode::Admin, HostMode::Space] {
        let host = claimed(mode);
        let encoded = serde_json::to_value(&host).unwrap();
        let restored: HostPolicy = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(restored, host);
        assert_eq!(restored.validate(mode), Ok(()));
        let other_mode = match mode {
            HostMode::Admin => HostMode::Space,
            HostMode::Space => HostMode::Admin,
        };
        assert_eq!(restored.validate(other_mode), Err(Denied::ModeChanged));
        let mut malformed = encoded;
        malformed["mode"] = serde_json::to_value(other_mode).unwrap();
        let malformed: HostPolicy = serde_json::from_value(malformed).unwrap();
        assert_eq!(malformed.validate(other_mode), Err(Denied::InvalidState));
    }
    let mut empty_admins = serde_json::to_value(claimed(HostMode::Admin)).unwrap();
    empty_admins["claim"]["userIds"] = serde_json::json!([]);
    let mut host: HostPolicy = serde_json::from_value(empty_admins).unwrap();
    assert_eq!(host.validate(HostMode::Admin), Err(Denied::InvalidState));
    assert_eq!(host.claim_verified(&user()), Err(Denied::InvalidState));
}
