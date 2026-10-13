use core_api::{
    client_identity::ClientPrincipal,
    space_access::{SpaceAccessFacts, SpaceAccessLevel as Level, SpaceMemberRole as Role},
};

#[test]
fn all_membership_and_delegation_combinations_obey_the_same_hierarchy() {
    let user = ClientPrincipal::User {
        user_id: "user".into(),
    };
    for (role, maximum) in [
        (None, None),
        (Some(Role::Member), Some(Level::Write)),
        (Some(Role::Owner), Some(Level::Manage)),
    ] {
        for ceiling in [
            None,
            Some(Level::Read),
            Some(Level::Write),
            Some(Level::Manage),
        ] {
            let facts = SpaceAccessFacts {
                principal: &user,
                tenant_id: "tenant",
                scope_id: "space",
                member_role: role,
                ceiling,
            };
            for required in [Level::Read, Level::Write, Level::Manage] {
                let expected = maximum.is_some_and(|level| level >= required)
                    && ceiling.is_none_or(|level| level >= required);
                assert_eq!(facts.allows(required), expected);
            }
        }
    }
}

#[test]
fn space_client_never_inherits_the_enrolling_users_owner_role() {
    let principal = ClientPrincipal::Space {
        tenant_id: "tenant".into(),
        scope_id: "space".into(),
    };
    for role in [None, Some(Role::Member), Some(Role::Owner)] {
        let facts = SpaceAccessFacts {
            principal: &principal,
            tenant_id: "tenant",
            scope_id: "space",
            member_role: role,
            ceiling: Some(Level::Manage),
        };
        assert!(facts.allows(Level::Read));
        assert!(facts.allows(Level::Write));
        assert!(!facts.allows(Level::Manage));
    }
}

#[test]
fn space_and_tenant_are_both_authorization_boundaries() {
    let principal = ClientPrincipal::Space {
        tenant_id: "tenant".into(),
        scope_id: "space".into(),
    };
    for (tenant, space) in [
        ("other", "space"),
        ("tenant", "other"),
        ("", "space"),
        ("tenant", " "),
    ] {
        let facts = SpaceAccessFacts {
            principal: &principal,
            tenant_id: tenant,
            scope_id: space,
            member_role: Some(Role::Owner),
            ceiling: None,
        };
        assert_eq!(facts.effective_level(), None);
    }
}

#[test]
fn read_only_delegation_restricts_a_space_client() {
    let principal = ClientPrincipal::Space {
        tenant_id: "tenant".into(),
        scope_id: "space".into(),
    };
    let facts = SpaceAccessFacts {
        principal: &principal,
        tenant_id: "tenant",
        scope_id: "space",
        member_role: None,
        ceiling: Some(Level::Read),
    };
    assert!(facts.allows(Level::Read));
    assert!(!facts.allows(Level::Write));
}

#[test]
fn an_empty_user_or_unknown_role_cannot_gain_access() {
    let principal = ClientPrincipal::User {
        user_id: " ".into(),
    };
    let facts = SpaceAccessFacts {
        principal: &principal,
        tenant_id: "tenant",
        scope_id: "space",
        member_role: Some(Role::Owner),
        ceiling: None,
    };
    assert!(!facts.allows(Level::Read));
    assert!(serde_json::from_str::<Role>("\"ADMIN\"").is_err());
    assert!(serde_json::from_str::<Level>("\"execute\"").is_err());
}
