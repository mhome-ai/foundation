use app_facade_api::{
    authorization::{required, OperationPolicy},
    client_space::*,
    routing::{route_policy_for_target, ExecutionSelector},
};
use serde_json::json;
#[test]
fn focus_is_connection_owned_and_requires_transport_proof() {
    assert_eq!(
        route_policy_for_target(FOCUS_TARGET).unwrap().execution,
        ExecutionSelector::Connection
    );
    assert_eq!(
        required(FOCUS_TARGET),
        Some(OperationPolicy::ClientConnection)
    );
    assert!(route_policy_for_target("/focus").is_none());
}
#[test]
fn strict_call_distinguishes_clear_from_missing_or_legacy_input() {
    for id in [Some("space-b"), None] {
        assert_eq!(
            FocusInput::parse(&focus_call(id))
                .unwrap()
                .0
                .target_space_id
                .as_deref(),
            id
        );
    }
    for payload in [
        "space-b",
        "",
        r#"{"control":{"mode":"direct"},"input":{}}"#,
        r#"{"control":{"mode":"direct"},"input":{"targetSpaceId":""}}"#,
        r#"{"control":{"mode":"prepare"},"input":{"targetSpaceId":"b"}}"#,
        r#"{"control":{"mode":"direct"},"input":{"targetSpaceId":"b","clientId":"other"}}"#,
    ] {
        assert!(FocusInput::parse(payload).is_err(), "{payload}");
    }
    assert_eq!(focus_data(None), json!({"focusedSpaceId":null}));
}

#[test]
fn both_presentations_keep_the_same_focus_outcome_and_false_acks_fail() {
    use app_facade_api::{response, ResponseDetail};
    let full =
        response::present(FOCUS_TARGET, focus_data(Some("b")), ResponseDetail::Full).unwrap();
    let essential = response::present(
        FOCUS_TARGET,
        focus_data(Some("b")),
        ResponseDetail::Essential,
    )
    .unwrap();
    let full = serde_json::to_value(full).unwrap();
    let essential = serde_json::to_value(essential).unwrap();
    assert_eq!(
        full,
        json!({"status":"success","data":{"focusedSpaceId":"b"}})
    );
    assert_eq!(
        essential,
        json!({"status":"success","data":"focusedSpaceId: b\n"})
    );
    require_success(&full.to_string()).unwrap();
    require_success(&essential.to_string()).unwrap();
    for bad in [
        "{}",
        r#"{"success":true}"#,
        r#"{"status":"error","error":{"code":"UNAUTHORIZED","message":"denied"}}"#,
    ] {
        assert!(require_success(bad).is_err());
    }
}
