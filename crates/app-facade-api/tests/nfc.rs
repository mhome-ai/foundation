use app_facade_api::{nfc::*, ResponseDetail};
use serde_json::{json, Value};

fn accepts(target: &str, input: Value) -> bool {
    match target {
        LIST => serde_json::from_value::<ListInput>(input).is_ok(),
        SOURCES_LIST => serde_json::from_value::<SourcesInput>(input).is_ok(),
        CREATE => serde_json::from_value::<CreateInput>(input)
            .map(|mut i| i.normalize().is_ok())
            .unwrap_or(false),
        UPDATE => serde_json::from_value::<UpdateInput>(input)
            .map(|mut i| i.normalize().is_ok())
            .unwrap_or(false),
        ACTIVATE => serde_json::from_value::<ActivateInput>(input)
            .map(|i| required(&i.tag_id, "tagId").is_ok())
            .unwrap_or(false),
        DELETE => serde_json::from_value::<DeleteInput>(input)
            .map(|mut i| i.normalize().is_ok())
            .unwrap_or(false),
        _ => panic!("unknown NFC target"),
    }
}
#[test]
fn nfc_shared_contract_cases() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/nfc.conformance.json")).unwrap();
    for case in cases {
        assert_eq!(
            accepts(case["target"].as_str().unwrap(), case["input"].clone()),
            case["valid"].as_bool().unwrap(),
            "{case}"
        );
    }
}
#[test]
fn nfc_permissions_routing_and_acknowledgements() {
    use app_facade_api::authorization::{required, OperationPolicy};
    use app_facade_api::routing::{route_policy_for_target, ExecutionSelector};
    assert_eq!(
        route_policy_for_target(SOURCES_LIST).unwrap().execution,
        ExecutionSelector::Cloud
    );
    for target in [LIST, CREATE, ACTIVATE, UPDATE, DELETE] {
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::ScopeMode
        );
    }
    assert_eq!(required(LIST), Some(OperationPolicy::SpaceRead));
    assert_eq!(required(SOURCES_LIST), Some(OperationPolicy::SpaceRead));
    for target in [CREATE, ACTIVATE, UPDATE, DELETE] {
        assert_eq!(required(target), Some(OperationPolicy::SpaceWrite));
    }
    for old in [
        "list",
        "sources",
        "create",
        "activate",
        "update",
        "batchDelete",
    ] {
        assert_eq!(required(&format!("/nfc/{old}")), None);
    }
    for target in [ACTIVATE, UPDATE, DELETE] {
        for detail in [ResponseDetail::Full, ResponseDetail::Essential] {
            let result =
                app_facade_api::response::present(target, json!({"success":true}), detail).unwrap();
            assert_eq!(
                serde_json::to_value(result).unwrap(),
                json!({"status":"success"})
            );
            assert!(
                app_facade_api::response::present(target, json!({"success":false}), detail)
                    .unwrap()
                    .is_error()
            );
        }
    }
}
#[test]
fn nfc_timestamp_presentation_keeps_unknown_zero() {
    let data = json!({"nfcTags":[{"tagId":"t1","createdAt":1730615400000_i64,"lastUsedAt":0}]});
    let output = app_facade_api::response::present_in_timezone(
        LIST,
        data.clone(),
        ResponseDetail::Essential,
        "Asia/Shanghai",
    )
    .unwrap();
    let value = serde_json::to_value(output).unwrap();
    let parsed: Value = serde_yaml::from_str(value["data"].as_str().unwrap()).unwrap();
    assert_eq!(parsed["nfcTags"][0]["lastUsedAt"], 0);
    assert!(parsed["nfcTags"][0]["createdAt"]
        .as_str()
        .unwrap()
        .contains("Asia/Shanghai"));
    assert_eq!(
        serde_json::to_value(
            app_facade_api::response::present(LIST, data.clone(), ResponseDetail::Full).unwrap()
        )
        .unwrap()["data"],
        data
    );
}
