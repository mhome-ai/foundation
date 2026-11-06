use app_facade_api::{authorization, delegation::DelegatedCall};
use core_api::{
    client_identity::ClientPrincipal,
    space_access::{SpaceAccessFacts, SpaceMemberRole},
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Case {
    name: String,
    principal: ClientPrincipal,
    member_role: Option<SpaceMemberRole>,
    tenant_id: String,
    scope_id: String,
    target: String,
    allowed: bool,
}

#[test]
fn shared_decision_matrix() {
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/authorization/decisions.json")).unwrap();
    for case in cases {
        let facts = SpaceAccessFacts {
            principal: &case.principal,
            tenant_id: &case.tenant_id,
            scope_id: &case.scope_id,
            member_role: case.member_role,
        };
        assert_eq!(
            authorization::required(&case.target).is_some_and(|policy| policy.allows(&facts)),
            case.allowed,
            "{}",
            case.name
        );
    }
}

#[test]
fn manifest_and_delegation_are_strict() {
    assert_eq!(
        authorization::manifest().contract_version,
        env!("CARGO_PKG_VERSION")
    );
    assert!(!authorization::allows_service("/app/hub/restart"));
    assert!(authorization::allows_service("/app/artifact/internal/read"));
    assert!(authorization::required("/app/plugin/unknown").is_none());
    assert!(serde_json::from_value::<DelegatedCall>(
        serde_json::json!({"control":{"mode":"direct"},"input":{}})
    )
    .is_err());
    assert!(serde_json::from_value::<DelegatedCall>(serde_json::json!({"authority":{"kind":"caller"},"call":{"control":{"mode":"direct"},"input":{}}})).is_err());
}

#[test]
fn current_conversation_and_talk_routes_are_all_declared() {
    for target in conversation_api::talk::REQUEST_TARGETS {
        assert!(
            authorization::required(target).is_some(),
            "Missing Talk policy: {target}"
        );
    }
    for target in [
        "/app/artifact/upload/start",
        "/app/artifact/upload/complete",
        "/app/artifact/upload/cancel",
    ] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceWrite)
        );
    }
    for target in [
        "/app/system/hosts/claim",
        "/app/widget/state/get",
        "/app/widget/brief/list",
        "/app/device/brief/list",
        "/integration/fix",
        "/scope/list",
        "/scope/get",
        "/scope/agent/get",
        "/scope/location/list",
        "/scope/preference/get",
    ] {
        assert!(
            authorization::required(target).is_none(),
            "Retired policy: {target}"
        );
    }
}

#[test]
fn delegated_wire_schema_and_rust_agree_on_authority() {
    use serde_json::{json, Value};
    let schema: Value =
        serde_json::from_str(include_str!("../schema/delegated-call.v1.schema.json")).unwrap();
    let call_schema: Value =
        serde_json::from_str(include_str!("../schema/facade-call.v1.schema.json")).unwrap();
    let validator = jsonschema::options()
        .with_resource(
            "https://schemas.mhome.ai/app-facade/facade-call.v1.schema.json",
            jsonschema::Resource::from_contents(call_schema).unwrap(),
        )
        .build(&schema)
        .unwrap();
    let call = json!({"control":{"mode":"direct"},"input":{}});
    for authority in [
        json!({"kind":"service"}),
        json!({"kind":"caller","principal":{"kind":"user","userId":"u"}}),
        json!({"kind":"caller","principal":{"kind":"space","tenantId":"t","scopeId":"s"}}),
    ] {
        let wire = json!({"authority":authority,"call":call});
        let typed: DelegatedCall = serde_json::from_value(wire.clone()).unwrap();
        assert!(validator.is_valid(&wire));
        assert_eq!(serde_json::to_value(typed).unwrap(), wire);
    }
    for wire in [
        call.clone(),
        json!({"authority":{"kind":"caller"},"call":call}),
        json!({"authority":{"kind":"service","principal":{"kind":"user","userId":"owner"}},"call":call}),
        json!({"authority":{"kind":"service"},"call":call,"invocation":{}}),
    ] {
        assert!(!validator.is_valid(&wire));
        assert!(serde_json::from_value::<DelegatedCall>(wire).is_err());
    }
}

#[test]
fn quickword_has_one_read_and_one_direct_write() {
    use authorization::OperationPolicy;
    assert_eq!(
        authorization::required("/app/quickword/list"),
        Some(OperationPolicy::SpaceRead)
    );
    assert_eq!(
        authorization::required("/app/quickword/run"),
        Some(OperationPolicy::SpaceWrite)
    );
    assert_eq!(authorization::required("/app/quickword/brief/list"), None);
    for target in [
        "/quickword/standard/list",
        "/quickword/standard/listRelatedAutomations",
    ] {
        assert_eq!(authorization::required(target), None, "{target}");
    }
}

#[test]
fn dashboard_summary_lists_use_canonical_paths_without_aliases() {
    assert_eq!(authorization::required("/app/dashboard/list"), None);
    for resource in ["canvas", "card"] {
        assert_eq!(
            authorization::required(&format!("/app/dashboard/{resource}/list")),
            Some(authorization::OperationPolicy::SpaceRead)
        );
        assert_eq!(
            authorization::required(&format!("/app/dashboard/{resource}/brief/list")),
            None
        );
    }
}

#[test]
fn dashboard_layout_read_is_space_read() {
    assert_eq!(
        authorization::required("/app/dashboard/canvas/layout/get"),
        authorization::required("/app/dashboard/card/list")
    );
    assert!(authorization::required("/app/dashboard/canvas/layout/get").is_some());
}

#[test]
fn widget_presentation_uses_space_reads_and_retires_the_old_controllers() {
    for target in [
        "/app/presentation/widget/list",
        "/app/presentation/widget/names/get",
        "/app/presentation/widget/defaults/get",
    ] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceRead)
        );
    }
    for target in [
        "/widget/list",
        "/widget/names",
        "/widget/default/get",
        "/widget/circadian/list",
    ] {
        assert!(authorization::required(target).is_none());
    }
}

#[test]
fn obsolete_widget_mutations_are_not_public_targets() {
    for target in ["/app/widget/upsert", "/ui/widgetCard/create"] {
        assert!(authorization::required(target).is_none());
    }
    for target in [
        "/app/widget/data/create",
        "/app/widget/button/create",
        "/app/widget/circadian/create",
        "/app/dashboard/card/add",
    ] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceWrite)
        );
    }
}

#[test]
fn room_has_only_the_canonical_public_facades() {
    for target in [
        "/app/room/list",
        "/app/room/create",
        "/app/room/rename",
        "/app/room/delete",
        "/app/room/reorder",
        "/app/room/devices/move",
        "/app/room/devices/remove",
    ] {
        assert_eq!(
            app_facade_api::routing::route_policy_for_target(target)
                .unwrap()
                .execution,
            app_facade_api::routing::ExecutionSelector::ScopeMode,
            "{target}"
        );
    }
    for target in [
        "/room/list",
        "/room/get",
        "/room/save",
        "/room/delete",
        "/room/reorder",
        "/room/moveDevice",
        "/room/removeDevices",
    ] {
        assert!(
            authorization::required(target).is_none(),
            "Retired Room policy: {target}"
        );
    }
    assert_eq!(
        authorization::required("/app/room/list"),
        Some(authorization::OperationPolicy::SpaceRead)
    );
    for target in [
        "/app/room/create",
        "/app/room/rename",
        "/app/room/delete",
        "/app/room/reorder",
        "/app/room/devices/move",
        "/app/room/devices/remove",
    ] {
        assert_eq!(
            authorization::required(target),
            Some(authorization::OperationPolicy::SpaceWrite)
        );
    }
}

#[test]
fn all_legacy_widget_requests_are_retired_and_runtime_control_is_a_write() {
    for target in [
        "/widget/create",
        "/widget/data/update",
        "/widget/button/trigger",
        "/widget/setting/update",
        "/widget/circadian/save",
        "/widget/circadian/updateConfig",
        "/widget/circadian/delete",
        "/widget/history/list",
        "/widget/batchDelete",
    ] {
        assert_eq!(authorization::required(target), None, "{target}");
    }
    assert_eq!(
        authorization::required("/app/widget/circadian/runtime/update"),
        Some(authorization::OperationPolicy::SpaceWrite)
    );
    assert!(app_facade_api::response::is_acknowledgement(
        "/app/widget/circadian/runtime/update"
    ));
}
