use app_facade_api::forwarding::{ActionInvocation, ForwardedCall, ForwardedResponse};
use serde_json::json;

#[test]
fn complete_forwarding_is_distinct_from_internal_dispatch() {
    let value = json!({"principal":{"kind":"user","userId":"user"},
        "caller":{"kind":"mcp","surfaceId":"mcp:grant","operationId":"op"},
        "call":{"control":{"mode":"commit","preparedActionId":"action","approvalToken":"token"},"input":null}});
    let call: ForwardedCall = serde_json::from_value(value.clone()).unwrap();
    call.call.validate().unwrap();
    assert_eq!(serde_json::to_value(call).unwrap(), value);
    assert!(
        serde_json::from_value::<ForwardedCall>(json!({"authority":{"kind":"service"},
        "call":{"control":{"mode":"direct"},"input":{}}}))
        .is_err()
    );
    let mut spoof = value;
    spoof["caller"]["threadId"] = json!("invented-thread");
    assert!(serde_json::from_value::<ForwardedCall>(spoof).is_err());
}

#[test]
fn projected_responses_stay_structured_and_invocation_kinds_remain_distinct() {
    assert!(
        serde_json::from_value::<ForwardedResponse>(json!({"kind":"text","value":"hello"}))
            .is_err()
    );
    let full = ForwardedResponse::Json(json!({"status":"success","data":"hello"}));
    let essential = ForwardedResponse::Json(json!({"status":"success","data":"hello\n"}));
    assert_ne!(
        serde_json::to_value(full).unwrap(),
        serde_json::to_value(essential).unwrap()
    );
    let mcp = ActionInvocation::Mcp {
        surface_id: "mcp:grant".into(),
        operation_id: "op".into(),
    };
    mcp.validate().unwrap();
    assert_eq!(mcp.thread_id(), None);
    assert_eq!(mcp.run_id(), None);
    assert!(
        ActionInvocation::Mcp {
            surface_id: " ".into(),
            operation_id: "op".into()
        }
        .validate()
        .is_err()
    );
}

#[test]
fn shared_routing_cases_match_the_rust_executor_contract() {
    use app_facade_api::routing::{ExecutionSelector, route_policy_for_target};
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("../fixtures/routing/cases.json")).unwrap();
    for case in cases {
        let actual = match route_policy_for_target(case["target"].as_str().unwrap())
            .unwrap()
            .execution
        {
            ExecutionSelector::Cloud => "cloud",
            ExecutionSelector::Hub => "hub",
            ExecutionSelector::ScopeMode => "scopeMode",
            ExecutionSelector::RequestPlacement => "requestPlacement",
        };
        assert_eq!(actual, case["execution"], "{case}");
    }
}
