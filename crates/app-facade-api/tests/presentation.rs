use app_facade_api::{response, ResponseDetail};
use serde_json::Value;
#[test]
fn shared_projection_and_outcome_contract() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/facade-response.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/presentation/cases.json")).unwrap();
    for case in cases {
        let target = case["target"].as_str().unwrap();
        let full = serde_json::to_value(
            response::present(target, case["data"].clone(), ResponseDetail::Full).unwrap(),
        )
        .unwrap();
        let essential = serde_json::to_value(
            response::present(target, case["data"].clone(), ResponseDetail::Essential).unwrap(),
        )
        .unwrap();
        for response in [&full, &essential] {
            validator.validate(response).unwrap();
            serde_json::from_value::<app_facade_api::FacadeResponse>(response.clone()).unwrap();
        }
        assert_eq!(full["status"], case["status"], "{}", case["name"]);
        assert_eq!(essential["status"], case["status"]);
        if case["status"] == "error" {
            assert_eq!(full["error"]["code"], case["code"]);
            assert_eq!(essential, full);
        } else {
            assert_eq!(full["data"], case["data"]);
            assert!(essential.get("data").is_none());
            let parsed: Value = serde_yaml::from_str(essential["text"].as_str().unwrap()).unwrap();
            assert_eq!(parsed, case["essential"], "{}", case["name"]);
        }
    }
}

#[test]
fn quickword_full_and_essential_preserve_parameter_contract_and_dispatch_result() {
    use serde_json::json;
    for (target, data) in [
        (
            "/app/quickword/list",
            json!({"scopeId":"space","quickWords":[
                {"quickWord":"Movie time","parameters":[]},
                {"quickWord":"Set ${mode}","parameters":[{"name":"mode","type":"String","allowedValues":["eco","comfort"]}]}
            ]}),
        ),
        (
            "/app/quickword/run",
            json!({"scopeId":"space","quickWord":"Set ${mode}","dispatched":true}),
        ),
    ] {
        let full = serde_json::to_value(
            response::present(target, data.clone(), ResponseDetail::Full).unwrap(),
        )
        .unwrap();
        let essential = serde_json::to_value(
            response::present(target, data.clone(), ResponseDetail::Essential).unwrap(),
        )
        .unwrap();
        assert_eq!(full["data"], data);
        assert!(essential.get("data").is_none());
        let parsed: Value = serde_yaml::from_str(essential["text"].as_str().unwrap()).unwrap();
        assert_eq!(parsed, data);
    }
}
