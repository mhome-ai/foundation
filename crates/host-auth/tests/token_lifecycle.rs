use host_auth::token_session::{
    renewal::{Coordinator, FailureKind, Work, ATTEMPT_TIMEOUT_MS, RETRY_MAX_MS, RETRY_MIN_MS},
    *,
};

const START: u64 = 1_800_000_000;
const DAY: u64 = 24 * 60 * 60;

fn binding() -> Binding {
    Binding {
        host_id: "host-a".into(),
        env: Default::default(),
        binding_generation: "binding-a".into(),
        controller_id: "client-a".into(),
        controller_kind: ControllerKind::UserClient,
        principal: Principal::User {
            user_id: "alice".into(),
        },
    }
}

fn lease(now: u64) -> AuthorizationLease {
    AuthorizationLease::from_verified_authority(now, now + 7 * DAY, now).unwrap()
}

fn session() -> (Session, AccessWindow) {
    Session::establish(binding(), "session-a".into(), lease(START), START).unwrap()
}

#[test]
fn local_refresh_keeps_offline_deadline_and_caps_each_access_token() {
    let (mut session, original) = session();
    assert_eq!(original.expires_at(), START + 3600);
    let deadline = START + 7 * DAY;
    for day in 1..7 {
        let now = START + day * DAY;
        let access = session.refresh_access(&binding(), now).unwrap();
        assert_eq!(access.expires_at(), now + 3600);
        assert_eq!(session.lease().valid_until(), deadline);
        assert_eq!(session.idle_expires_at().unwrap(), now + 30 * DAY);
    }
    let last = session.refresh_access(&binding(), deadline - 20).unwrap();
    assert_eq!(last.expires_at(), deadline);
    let before = session.clone();
    assert_eq!(
        session.refresh_access(&binding(), deadline),
        Err(Failure::AuthorizationExpired)
    );
    assert_eq!(session, before, "failed refresh must not touch idle expiry");
    assert_eq!(
        session.authorize(&binding(), &last, deadline),
        Err(Failure::AuthorizationExpired)
    );
}

#[test]
fn ten_day_offline_session_recovers_with_new_authority_without_rebinding() {
    let (mut session, original) = session();
    let now = START + 10 * DAY;
    assert_eq!(
        session
            .maintenance(&binding(), Some(&original), now)
            .unwrap(),
        Maintenance {
            access: AccessAction::RenewAuthorization,
            review_in_background: false,
        }
    );
    session
        .renew_authorization(&binding(), lease(now), now)
        .unwrap();
    assert_eq!(session.session_id(), "session-a");
    assert_eq!(session.binding(), &binding());
    assert_eq!(session.idle_expires_at().unwrap(), START + 30 * DAY);
    assert_eq!(
        session.authorize(&binding(), &original, now),
        Err(Failure::AccessExpired),
        "online renewal cannot extend an existing access token"
    );
    let access = session.refresh_access(&binding(), now).unwrap();
    assert!(session.authorize(&binding(), &access, now).is_ok());
    assert_eq!(session.lease().valid_until(), now + 7 * DAY);
}

#[test]
fn thirty_day_idle_expiry_requires_a_new_session_but_not_a_new_binding() {
    let (mut old, access) = session();
    let now = START + 30 * DAY;
    assert_eq!(
        old.maintenance(&binding(), Some(&access), now)
            .unwrap()
            .access,
        AccessAction::EstablishSession
    );
    assert_eq!(
        old.renew_authorization(&binding(), lease(now), now),
        Err(Failure::SessionExpired)
    );
    assert_eq!(
        old.refresh_access(&binding(), now),
        Err(Failure::SessionExpired)
    );
    // An adapter obtains fresh authority using an independent, still-valid
    // upstream identity. No expired Host RT is accepted as that evidence.
    let (new, access) = Session::establish(binding(), "session-b".into(), lease(now), now).unwrap();
    assert_eq!(new.binding(), old.binding());
    assert_ne!(new.session_id(), old.session_id());
    assert!(new.authorize(&binding(), &access, now).is_ok());
}

#[test]
fn online_reviews_alone_cannot_keep_an_unused_refresh_session_alive() {
    let (mut session, _) = session();
    for day in 1..30 {
        let now = START + day * DAY;
        session
            .renew_authorization(&binding(), lease(now), now)
            .unwrap();
    }
    assert_eq!(session.idle_expires_at().unwrap(), START + 30 * DAY);
    assert_eq!(
        session.refresh_access(&binding(), START + 30 * DAY),
        Err(Failure::SessionExpired)
    );
}

#[test]
fn restart_roundtrip_does_not_reset_any_deadline() {
    let (mut session, _) = session();
    session.refresh_access(&binding(), START + DAY).unwrap();
    let saved = serde_json::to_vec(&session).unwrap();
    let restored: Session = serde_json::from_slice(&saved).unwrap();
    assert_eq!(session, restored);
    assert_eq!(restored.lease().valid_until(), START + 7 * DAY);
    assert_eq!(restored.idle_expires_at().unwrap(), START + 31 * DAY);
    assert_eq!(
        restored
            .maintenance(&binding(), None, START + 8 * DAY)
            .unwrap()
            .access,
        AccessAction::RenewAuthorization
    );
}

#[test]
fn valid_lan_access_is_independent_of_daily_online_review() {
    let (mut session, _) = session();
    let now = START + DAY - 1;
    let access = session.refresh_access(&binding(), now).unwrap();
    assert_eq!(
        session.maintenance(&binding(), Some(&access), now).unwrap(),
        Maintenance {
            access: AccessAction::Ready,
            review_in_background: false,
        }
    );
    assert_eq!(
        session
            .maintenance(&binding(), Some(&access), now + 1)
            .unwrap(),
        Maintenance {
            access: AccessAction::Ready,
            review_in_background: true,
        }
    );
    assert_eq!(
        session
            .maintenance(&binding(), Some(&access), now + 3300)
            .unwrap()
            .access,
        AccessAction::RefreshAccess
    );
    session
        .renew_authorization(&binding(), lease(now + 1), now + 1)
        .unwrap();
    assert!(
        !session
            .maintenance(&binding(), Some(&access), now + 1)
            .unwrap()
            .review_in_background
    );
}

#[test]
fn delegated_authority_never_exceeds_the_parent_deadline() {
    let confirmed = START;
    let now = START + 6 * DAY;
    let delegated =
        AuthorizationLease::from_verified_authority(confirmed, START + 7 * DAY, now).unwrap();
    let (_, access) = Session::establish(binding(), "child".into(), delegated, now).unwrap();
    assert_eq!(delegated.valid_until(), START + 7 * DAY);
    assert_eq!(access.expires_at(), now + 3600);
    let shortened = AuthorizationLease::from_verified_authority(START, START + 100, START).unwrap();
    let (_, access) = Session::establish(binding(), "short".into(), shortened, START).unwrap();
    assert_eq!(access.expires_at(), START + 100);
    let excessive =
        AuthorizationLease::from_verified_authority(START, START + 99 * DAY, START).unwrap();
    assert_eq!(excessive.valid_until(), START + 7 * DAY);
}

#[test]
fn revoked_sessions_cannot_refresh_renew_or_suggest_automatic_reenrollment() {
    let (mut session, access) = session();
    session.revoke();
    for now in [START, START + 8 * DAY, START + 40 * DAY] {
        assert_eq!(
            session.authorize(&binding(), &access, now),
            Err(Failure::Revoked)
        );
        assert_eq!(
            session.refresh_access(&binding(), now),
            Err(Failure::Revoked)
        );
        assert_eq!(
            session.renew_authorization(&binding(), lease(now), now),
            Err(Failure::Revoked)
        );
        assert_eq!(
            session.maintenance(&binding(), Some(&access), now),
            Err(Failure::Revoked)
        );
    }
    let restored: Session = serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
    assert_eq!(
        restored.authorize(&binding(), &access, START),
        Err(Failure::Revoked)
    );
}

#[test]
fn renewal_rejects_scope_user_controller_host_and_generation_changes() {
    let (mut session, access) = session();
    let mut cases = Vec::new();
    let mut changed = binding();
    changed.principal = Principal::User {
        user_id: "bob".into(),
    };
    cases.push(changed);
    let mut changed = binding();
    changed.principal = Principal::Space {
        tenant_id: "tenant".into(),
        space_id: "home".into(),
    };
    cases.push(changed);
    let mut changed = binding();
    changed.controller_id = "other-client".into();
    cases.push(changed);
    let mut changed = binding();
    changed.controller_kind = ControllerKind::Core;
    cases.push(changed);
    let mut changed = binding();
    changed.host_id = "other-host".into();
    cases.push(changed);
    let mut changed = binding();
    changed.binding_generation = "reset-host".into();
    cases.push(changed);
    for changed in cases {
        assert_eq!(
            session.authorize(&changed, &access, START),
            Err(Failure::BindingMismatch)
        );
        assert_eq!(
            session.refresh_access(&changed, START),
            Err(Failure::BindingMismatch)
        );
        assert_eq!(
            session.renew_authorization(&changed, lease(START), START),
            Err(Failure::BindingMismatch)
        );
    }
}

#[test]
fn stale_online_result_and_clock_rollback_cannot_extend_a_session() {
    let (mut session, _) = session();
    session
        .renew_authorization(&binding(), lease(START + DAY), START + DAY)
        .unwrap();
    let before = session.clone();
    assert_eq!(
        session.renew_authorization(&binding(), lease(START), START + DAY),
        Err(Failure::StaleLease)
    );
    assert_eq!(session, before);
    assert_eq!(
        session.refresh_access(&binding(), START),
        Err(Failure::ClockInvalid)
    );
    session
        .refresh_access(&binding(), START + DAY + 100)
        .unwrap();
    assert_eq!(
        session.refresh_access(&binding(), START + DAY),
        Err(Failure::ClockInvalid)
    );
    assert_eq!(
        AuthorizationLease::from_verified_authority(START + 1, START + 100, START),
        Err(Failure::ClockInvalid)
    );
    assert!(AuthorizationLease::from_verified_authority(u64::MAX, u64::MAX, u64::MAX).is_err());
}

#[test]
fn coordinator_single_flight_lanes_do_not_block_each_other() {
    let mut coordinator = Coordinator::new(42);
    let online = coordinator
        .begin(Work::RenewAuthorization, 0)
        .unwrap()
        .unwrap();
    assert!(coordinator
        .begin(Work::RenewAuthorization, 1)
        .unwrap()
        .is_none());
    let lan = coordinator.begin(Work::RefreshAccess, 1).unwrap().unwrap();
    assert_eq!(
        coordinator
            .succeed(lan, 2, |generation| {
                assert_eq!(generation, 42);
                Ok("access")
            })
            .unwrap(),
        Some("access")
    );
    assert!(coordinator.fail(online, FailureKind::Transient, 3).unwrap());
    assert!(coordinator
        .begin(Work::RenewAuthorization, 4)
        .unwrap()
        .is_none());
    assert!(coordinator.begin(Work::RefreshAccess, 4).unwrap().is_some());
}

#[test]
fn transient_retry_is_bounded_and_reconnect_does_not_duplicate_work() {
    let mut coordinator = Coordinator::new(1);
    let first = coordinator
        .begin(Work::RenewAuthorization, 0)
        .unwrap()
        .unwrap();
    coordinator.fail(first, FailureKind::Transient, 0).unwrap();
    assert!(coordinator
        .begin(Work::RenewAuthorization, RETRY_MIN_MS - 1)
        .unwrap()
        .is_none());
    let second = coordinator
        .begin(Work::RenewAuthorization, RETRY_MIN_MS * 2)
        .unwrap()
        .unwrap();
    coordinator
        .connection_restored(RETRY_MIN_MS * 2 + 1)
        .unwrap();
    assert!(coordinator
        .begin(Work::RenewAuthorization, RETRY_MIN_MS * 2 + 1)
        .unwrap()
        .is_none());
    coordinator
        .fail(second, FailureKind::Transient, RETRY_MIN_MS * 2 + 2)
        .unwrap();
    let mut now = RETRY_MIN_MS * 2 + 2;
    for _ in 0..20 {
        now += RETRY_MAX_MS;
        let attempt = coordinator
            .begin(Work::RenewAuthorization, now)
            .unwrap()
            .unwrap();
        coordinator
            .fail(attempt, FailureKind::Transient, now)
            .unwrap();
        assert!(coordinator
            .begin(Work::RenewAuthorization, now)
            .unwrap()
            .is_none());
    }
    coordinator.connection_restored(now + 1).unwrap();
    assert!(coordinator
        .begin(Work::RenewAuthorization, now + 1)
        .unwrap()
        .is_some());
}

#[test]
fn logout_and_other_owners_discard_late_results_without_publishing() {
    let mut old = Coordinator::new(1);
    let late = old.begin(Work::RefreshAccess, 0).unwrap().unwrap();
    old.retire();
    assert_eq!(
        old.succeed(late, 1, |_| -> anyhow::Result<()> {
            panic!("retired owner published")
        })
        .unwrap(),
        None
    );
    assert!(old.begin(Work::EstablishSession, 2).unwrap().is_none());
    let mut one = Coordinator::new(1);
    let foreign = one.begin(Work::RefreshAccess, 0).unwrap().unwrap();
    // Even matching generation numbers do not make tickets interchangeable.
    let mut two = Coordinator::new(1);
    let _own = two.begin(Work::RefreshAccess, 0).unwrap().unwrap();
    assert_eq!(
        two.succeed(foreign, 1, |_| -> anyhow::Result<()> {
            panic!("foreign owner published")
        })
        .unwrap(),
        None
    );
}

#[test]
fn expired_attempts_and_old_session_results_cannot_publish() {
    let mut coordinator = Coordinator::new(1);
    let old = coordinator.begin(Work::RefreshAccess, 0).unwrap().unwrap();
    assert_eq!(
        coordinator
            .succeed(old, ATTEMPT_TIMEOUT_MS, |_| -> anyhow::Result<()> {
                panic!("expired task published")
            })
            .unwrap(),
        None
    );
    assert!(coordinator
        .begin(Work::RefreshAccess, ATTEMPT_TIMEOUT_MS)
        .unwrap()
        .is_none());
    let now = ATTEMPT_TIMEOUT_MS + RETRY_MAX_MS;
    let late = coordinator
        .begin(Work::RefreshAccess, now)
        .unwrap()
        .unwrap();
    let establish = coordinator
        .begin(Work::EstablishSession, now)
        .unwrap()
        .unwrap();
    assert_eq!(
        coordinator.succeed(establish, now + 1, |_| Ok(())).unwrap(),
        Some(())
    );
    assert_eq!(
        coordinator
            .succeed(late, now + 2, |_| -> anyhow::Result<()> {
                panic!("old session published")
            })
            .unwrap(),
        None
    );
}

#[test]
fn login_restoration_retries_upstream_but_cannot_clear_revocation() {
    let mut coordinator = Coordinator::new(1);
    let attempt = coordinator
        .begin(Work::RenewAuthorization, 0)
        .unwrap()
        .unwrap();
    coordinator
        .fail(attempt, FailureKind::LoginRequired, 1)
        .unwrap();
    assert!(coordinator.needs_attention());
    coordinator.connection_restored(2).unwrap();
    assert!(coordinator
        .begin(Work::RenewAuthorization, 2)
        .unwrap()
        .is_none());
    let lan = coordinator.begin(Work::RefreshAccess, 2).unwrap().unwrap();
    assert_eq!(coordinator.succeed(lan, 3, |_| Ok(())).unwrap(), Some(()));
    coordinator.upstream_identity_restored(4).unwrap();
    assert!(!coordinator.needs_attention());
    let attempt = coordinator
        .begin(Work::RenewAuthorization, 4)
        .unwrap()
        .unwrap();
    coordinator.fail(attempt, FailureKind::Revoked, 5).unwrap();
    coordinator.upstream_identity_restored(6).unwrap();
    for work in [
        Work::RenewAuthorization,
        Work::RefreshAccess,
        Work::EstablishSession,
    ] {
        assert!(coordinator.begin(work, 6).unwrap().is_none());
    }
    assert!(coordinator.needs_attention());
}

#[test]
fn failed_atomic_publish_is_retried_and_monotonic_clock_is_enforced() {
    let mut coordinator = Coordinator::new(8);
    let attempt = coordinator
        .begin(Work::RenewAuthorization, 100)
        .unwrap()
        .unwrap();
    assert!(coordinator
        .succeed(attempt, 101, |_| -> anyhow::Result<()> {
            anyhow::bail!("storage failed")
        })
        .is_err());
    assert!(coordinator
        .begin(Work::RenewAuthorization, 102)
        .unwrap()
        .is_none());
    assert!(coordinator.begin(Work::RenewAuthorization, 99).is_err());
    assert!(coordinator
        .begin(Work::RenewAuthorization, 101 + RETRY_MAX_MS)
        .unwrap()
        .is_some());
}

#[test]
fn access_capped_at_lease_deadline_does_not_refresh_on_every_request() {
    let (mut session, _) = session();
    let now = START + 7 * DAY - 100;
    let access = session.refresh_access(&binding(), now).unwrap();
    for offset in 0..100 {
        assert_eq!(
            session
                .maintenance(&binding(), Some(&access), now + offset)
                .unwrap(),
            Maintenance {
                access: AccessAction::Ready,
                review_in_background: true,
            }
        );
    }
    assert_eq!(
        session
            .maintenance(&binding(), Some(&access), now + 100)
            .unwrap()
            .access,
        AccessAction::RenewAuthorization
    );
}

#[test]
fn space_sessions_share_lifecycle_but_never_cross_tenant_or_space() {
    let mut home = binding();
    home.controller_kind = ControllerKind::SpaceClient;
    home.principal = Principal::Space {
        tenant_id: "tenant-a".into(),
        space_id: "home-a".into(),
    };
    let (mut session, access) =
        Session::establish(home.clone(), "space-session".into(), lease(START), START).unwrap();
    assert!(session.authorize(&home, &access, START).is_ok());
    for (tenant, space) in [("tenant-b", "home-a"), ("tenant-a", "home-b")] {
        let mut other = home.clone();
        other.principal = Principal::Space {
            tenant_id: tenant.into(),
            space_id: space.into(),
        };
        assert_eq!(
            session.refresh_access(&other, START),
            Err(Failure::BindingMismatch)
        );
    }
    let now = START + 10 * DAY;
    session.renew_authorization(&home, lease(now), now).unwrap();
    let access = session.refresh_access(&home, now).unwrap();
    assert!(session.authorize(&home, &access, now).is_ok());
    assert_eq!(
        session.authorize(&binding(), &access, now),
        Err(Failure::BindingMismatch)
    );
}

#[test]
fn shared_pod_cannot_refresh_or_renew_as_core() {
    let mut shared = binding();
    shared.controller_kind = ControllerKind::SpaceClient;
    shared.principal = Principal::Space {
        tenant_id: "tenant-a".into(),
        space_id: "home-a".into(),
    };
    let (session, access) =
        Session::establish(shared.clone(), "pod-session".into(), lease(START), START).unwrap();
    let mut restored: Session =
        serde_json::from_slice(&serde_json::to_vec(&session).unwrap()).unwrap();
    let mut core = shared;
    core.controller_kind = ControllerKind::Core;
    assert_eq!(
        restored.authorize(&core, &access, START),
        Err(Failure::BindingMismatch)
    );
    assert_eq!(
        restored.refresh_access(&core, START),
        Err(Failure::BindingMismatch)
    );
    assert_eq!(
        restored.renew_authorization(&core, lease(START), START),
        Err(Failure::BindingMismatch)
    );
    assert_eq!(restored, session);
}

#[test]
fn controller_kind_is_required_and_cannot_mismatch_the_principal() {
    let mut old = serde_json::to_value(binding()).unwrap();
    old.as_object_mut().unwrap().remove("controllerKind");
    assert!(serde_json::from_value::<Binding>(old).is_err());

    let mut wrong_user = binding();
    wrong_user.controller_kind = ControllerKind::SpaceClient;
    let mut wrong_space = binding();
    wrong_space.principal = Principal::Space {
        tenant_id: "tenant-a".into(),
        space_id: "home-a".into(),
    };
    for invalid in [wrong_user, wrong_space] {
        assert_eq!(
            Session::establish(invalid, "session-a".into(), lease(START), START),
            Err(Failure::BindingMismatch)
        );
    }
}

#[test]
fn malformed_restored_deadlines_fail_closed() {
    let (session, access) = session();
    let mut json = serde_json::to_value(session).unwrap();
    json["lease"]["validUntil"] = (START + 8 * DAY).into();
    let restored: Session = serde_json::from_value(json).unwrap();
    assert_eq!(
        restored.authorize(&binding(), &access, START),
        Err(Failure::InvalidLease)
    );
    let malformed: AccessWindow = serde_json::from_value(serde_json::json!({
        "issuedAt": START,
        "expiresAt": START + 3601,
    }))
    .unwrap();
    let (session, _) = Session::establish(binding(), "new".into(), lease(START), START).unwrap();
    assert_eq!(
        session.authorize(&binding(), &malformed, START),
        Err(Failure::AccessExpired)
    );
}

#[test]
fn persisted_generation_check_prevents_cross_process_logout_resurrection() {
    let mut coordinator = Coordinator::new(7);
    let late = coordinator.begin(Work::RefreshAccess, 0).unwrap().unwrap();
    // Another process logged out and atomically advanced the durable generation.
    // It cannot retire this process's in-memory coordinator, so the publication
    // transaction also fences on the supplied owner generation.
    let persisted_generation = 8;
    let mut published = false;
    assert!(coordinator
        .succeed(late, 1, |captured_generation| {
            anyhow::ensure!(captured_generation == persisted_generation, "owner retired");
            published = true;
            Ok(())
        })
        .is_err());
    assert!(!published);
}
