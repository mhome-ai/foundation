use serde_json::{json, Value};

#[test]
fn only_catalog_cards_are_pinnable_and_all_required_entries_survive() {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/user-navigation.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/definitions/updateRequest");
    let validator = jsonschema::validator_for(&schema).unwrap();
    for layout in ["compact", "wide"] {
        assert!(validator
            .is_valid(&json!({"layout":layout,"cardIds":["widgets","menu","dashboard","now"]})));
    }
    for invalid in [
        json!({"layout":"phone","cardIds":["dashboard","now","menu"]}),
        json!({"layout":"compact","cardIds":["dashboard","now"]}),
        json!({"layout":"compact","cardIds":["dashboard","now","menu","menu"]}),
        json!({"layout":"compact","cardIds":["dashboard","now","menu","/entity/secret"]}),
        json!({"layout":"compact","cardIds":["dashboard","now","menu"],"userId":"another-user"}),
    ] {
        assert!(!validator.is_valid(&invalid), "{invalid}");
    }
    let catalog: Value =
        serde_json::from_str(include_str!("../manifest/navigation-cards.v1.json")).unwrap();
    assert_eq!(
        catalog["cardIds"],
        schema["definitions"]["cardIds"]["items"]["enum"]
    );
    assert_eq!(
        catalog["requiredCardIds"],
        json!(["dashboard", "now", "menu"])
    );
}

#[test]
fn menu_order_is_additive_and_only_accepts_optional_catalog_cards() {
    let mut schema: Value =
        serde_json::from_str(include_str!("../schema/user-navigation.v1.schema.json")).unwrap();
    schema["$ref"] = json!("#/definitions/updateRequest");
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&json!({
        "layout":"wide", "cardIds":["dashboard","now","menu"],
        "menuCardIds":["widgets","device","phone-setting"]
    })));
    for menu in [
        json!(["dashboard"]),
        json!(["menu"]),
        json!(["widgets", "widgets"]),
        json!(["/arbitrary"]),
        Value::Null,
    ] {
        assert!(!validator.is_valid(&json!({
            "layout":"wide", "cardIds":["dashboard","now","menu"], "menuCardIds":menu
        })));
    }
}

#[test]
fn navigation_is_a_user_cloud_contract() {
    let manifest: Value =
        serde_json::from_str(include_str!("../manifest/app-facade.v1.json")).unwrap();
    let auth: Value =
        serde_json::from_str(include_str!("../manifest/authorization.v1.json")).unwrap();
    let domain = &manifest["domains"]["userNavigation"];
    for target in domain["requestTargets"].as_array().unwrap() {
        let expected = if target.as_str().unwrap().ends_with("update") {
            "userWrite"
        } else {
            "userRead"
        };
        assert_eq!(auth["operations"][target.as_str().unwrap()], expected);
    }
}

#[test]
fn only_an_authenticated_user_can_read_or_write_personal_navigation() {
    use app_facade_api::{
        authorization,
        routing::{route_policy_for_target, ExecutionSelector},
    };
    use core_api::{client_identity::ClientPrincipal, space_access::SpaceAccessFacts};
    let user = ClientPrincipal::User {
        user_id: "alice".into(),
    };
    let space = ClientPrincipal::Space {
        tenant_id: "tenant".into(),
        scope_id: "home".into(),
    };
    for target in [
        "/app/user/navigation/catalog",
        "/app/user/navigation/get",
        "/app/user/navigation/update",
    ] {
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::Cloud
        );
        let policy = authorization::required(target).unwrap();
        assert!(policy.allows(&SpaceAccessFacts {
            principal: &user,
            tenant_id: "",
            scope_id: "",
            member_role: None
        }));
        assert!(!policy.allows(&SpaceAccessFacts {
            principal: &space,
            tenant_id: "tenant",
            scope_id: "home",
            member_role: None
        }));
    }
}
