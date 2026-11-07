use app_facade_api::{
    authorization::{required, OperationPolicy},
    billing::*,
    routing::{route_policy_for_target, ExecutionSelector},
    ResponseDetail,
};
use serde_json::Value;
#[test]
fn billing_uses_user_identity_cloud_routing_and_shared_projection() {
    let f: Value =
        serde_json::from_str(include_str!("../fixtures/billing.conformance.json")).unwrap();
    for (key, target) in [
        ("summary", SUMMARY),
        ("payments", PAYMENT_LIST),
        ("creditUsage", CREDIT_USAGE),
    ] {
        assert_eq!(required(target), Some(OperationPolicy::UserRead));
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::Cloud
        );
        let full = serde_json::to_value(
            app_facade_api::response::present(target, f[key].clone(), ResponseDetail::Full)
                .unwrap(),
        )
        .unwrap();
        let essential = serde_json::to_value(
            app_facade_api::response::present(target, f[key].clone(), ResponseDetail::Essential)
                .unwrap(),
        )
        .unwrap();
        assert_eq!(full["data"], f[key]);
        assert_eq!(
            serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
            f[key]
        );
    }
    for target in [
        "/user/usage",
        "/user/requestBalance",
        "/user/payment/history",
        "/user/credit/usage",
    ] {
        assert!(required(target).is_none());
    }
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<Summary>(f["summary"].clone()).unwrap())
            .unwrap(),
        f["summary"]
    );
    assert_eq!(
        serde_json::to_value(serde_json::from_value::<PaymentList>(f["payments"].clone()).unwrap())
            .unwrap(),
        f["payments"]
    );
    assert_eq!(
        serde_json::to_value(
            serde_json::from_value::<CreditUsage>(f["creditUsage"].clone()).unwrap()
        )
        .unwrap(),
        f["creditUsage"]
    );
    for (key, usage) in [("emptyInput", false), ("usageInput", true)] {
        for (kind, valid) in [("valid", true), ("invalid", false)] {
            for v in f[key][kind].as_array().unwrap() {
                let ok = if usage {
                    serde_json::from_value::<CreditUsageInput>(v.clone()).is_ok()
                } else {
                    serde_json::from_value::<EmptyInput>(v.clone()).is_ok()
                };
                assert_eq!(ok, valid, "{v}");
            }
        }
    }
}

#[test]
fn billing_schema_and_fixture_inputs_agree() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/billing.v1.schema.json")).unwrap();
    let fixtures: Value =
        serde_json::from_str(include_str!("../fixtures/billing.conformance.json")).unwrap();
    for key in ["emptyInput", "usageInput"] {
        let mut root = schema.clone();
        root["$ref"] = Value::String(format!("#/$defs/{key}"));
        let validator = jsonschema::validator_for(&root).unwrap();
        for (kind, valid) in [("valid", true), ("invalid", false)] {
            for value in fixtures[key][kind].as_array().unwrap() {
                assert_eq!(validator.is_valid(value), valid, "{key}: {value}");
            }
        }
    }
    for key in ["summary", "payments", "creditUsage"] {
        let mut root = schema.clone();
        root["$ref"] = Value::String(format!("#/$defs/{key}"));
        assert!(jsonschema::validator_for(&root)
            .unwrap()
            .is_valid(&fixtures[key]));
    }
}
