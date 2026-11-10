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
            f["essential"][key]
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
    for key in ["emptyInput", "usageInput", "productInput"] {
        let mut root = schema.clone();
        root["$ref"] = Value::String(format!("#/$defs/{key}"));
        let validator = jsonschema::validator_for(&root).unwrap();
        for (kind, valid) in [("valid", true), ("invalid", false)] {
            for value in fixtures[key][kind].as_array().unwrap() {
                assert_eq!(validator.is_valid(value), valid, "{key}: {value}");
            }
        }
    }
    for key in [
        "summary",
        "payments",
        "creditUsage",
        "checkout",
        "subscription",
    ] {
        let mut root = schema.clone();
        root["$ref"] = Value::String(format!("#/$defs/{key}"));
        assert!(jsonschema::validator_for(&root)
            .unwrap()
            .is_valid(&fixtures[key]));
    }
}

#[test]
fn subscription_commands_are_user_writes_in_cloud_with_shared_results() {
    let f: Value =
        serde_json::from_str(include_str!("../fixtures/billing.conformance.json")).unwrap();
    for (target, key) in [
        (CHECKOUT_CREATE, "checkout"),
        (SUBSCRIPTION_CANCEL, "subscription"),
        (SUBSCRIPTION_RESUME, "subscription"),
        (SUBSCRIPTION_CHANGE, "subscription"),
    ] {
        assert_eq!(required(target), Some(OperationPolicy::UserWrite));
        assert_eq!(
            route_policy_for_target(target).unwrap().execution,
            ExecutionSelector::Cloud
        );
        for detail in [ResponseDetail::Full, ResponseDetail::Essential] {
            let result = serde_json::to_value(
                app_facade_api::response::present(target, f[key].clone(), detail).unwrap(),
            )
            .unwrap();
            let data = if detail == ResponseDetail::Full {
                result["data"].clone()
            } else {
                serde_yaml::from_str::<Value>(result["data"].as_str().unwrap()).unwrap()
            };
            assert_eq!(
                data,
                if detail == ResponseDetail::Full {
                    &f[key]
                } else {
                    &f["essential"][key]
                }
                .clone()
            );
        }
    }
    for (kind, valid) in [("valid", true), ("invalid", false)] {
        for input in f["productInput"][kind].as_array().unwrap() {
            assert_eq!(
                serde_json::from_value::<ProductInput>(input.clone()).is_ok(),
                valid,
                "{input}"
            );
        }
    }
    let state: Subscription = serde_json::from_value(f["subscription"].clone()).unwrap();
    assert_eq!(serde_json::to_value(state).unwrap(), f["subscription"]);
    let checkout: Checkout = serde_json::from_value(f["checkout"].clone()).unwrap();
    assert_eq!(serde_json::to_value(checkout).unwrap(), f["checkout"]);
}
