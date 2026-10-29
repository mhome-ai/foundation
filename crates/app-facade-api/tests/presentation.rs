use app_facade_api::{response, ResponseDetail};
use serde_json::Value;

#[test]
fn card_skin_projection_fixtures_match_the_business_wire_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/card-skins.v1.schema.json")).unwrap();
    let package: Value =
        serde_json::from_str(include_str!("../schema/card-skin-package.v1.schema.json")).unwrap();
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/presentation/cases.json")).unwrap();
    for (name, definition) in [
        ("card-skin-release-list", "releaseList"),
        ("card-skin-release-detail", "release"),
        ("card-skin-selection", "selectionView"),
    ] {
        let case = cases.iter().find(|case| case["name"] == name).unwrap();
        let entry = serde_json::json!({"$ref": format!(
            "https://schemas.meowlink.app/card-skins.v1.json#/definitions/{definition}"
        )});
        let validator = jsonschema::options()
            .with_resource(
                schema["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(schema.clone()).unwrap(),
            )
            .with_resource(
                package["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(package.clone()).unwrap(),
            )
            .build(&entry)
            .unwrap();
        validator.validate(&case["data"]).unwrap();
    }
}

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
            let parsed: Value = serde_yaml::from_str(essential["data"].as_str().unwrap()).unwrap();
            assert_eq!(parsed, *case.get("essential").unwrap_or(&full));
        } else if case["acknowledgement"] == true {
            assert_eq!(full, serde_json::json!({"status":"success"}));
            assert_eq!(essential, full);
        } else {
            assert_eq!(full["data"], case["data"]);
            assert!(essential["data"].is_string());
            let parsed: Value = serde_yaml::from_str(essential["data"].as_str().unwrap()).unwrap();
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
            json!({"spaceId":"space","quickWords":[
                {"quickWord":"Movie time","parameters":[]},
                {"quickWord":"Set ${mode}","parameters":[{"name":"mode","type":"String","allowedValues":["eco","comfort"]}]}
            ]}),
        ),
        (
            "/app/quickword/run",
            json!({"spaceId":"space","quickWord":"Set ${mode}","dispatched":true}),
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
        assert!(essential["data"].is_string());
        let parsed: Value = serde_yaml::from_str(essential["data"].as_str().unwrap()).unwrap();
        assert_eq!(parsed, data);
    }
}
