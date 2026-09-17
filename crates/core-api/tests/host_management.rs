use core_api::host::management::{HostRuntimeAction as A, HostRuntimeRequest as R};
use serde_json::Value;
#[test]
fn native_requests_match_shared_schema_and_action_manifest() {
    let schema: Value = serde_json::from_str(include_str!(
        "../schema/host-management-request.v1.schema.json"
    ))
    .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let manifest: Value =
        serde_json::from_str(include_str!("../contract/host-management-v1.json")).unwrap();
    let actions = [
        A::Inspect,
        A::Metrics,
        A::Plan {
            components: vec!["core"],
            all: false,
        },
        A::Start {
            plan_id: "plan".into(),
        },
        A::Operation {
            operation_id: "op".into(),
        },
        A::RestartOperation {
            component: "core",
            operation_id: "op".into(),
        },
    ];
    assert_eq!(
        actions.len(),
        manifest["actions"].as_object().unwrap().len()
    );
    for action in actions {
        let value = serde_json::to_value(R {
            host_id: "host".into(),
            action,
        })
        .unwrap();
        assert!(validator.is_valid(&value), "{value}");
        assert!(manifest["actions"]
            .get(value["action"].as_str().unwrap())
            .is_some());
        let mut scoped = value;
        scoped["scopeId"] = "space".into();
        assert!(!validator.is_valid(&scoped));
    }
}
