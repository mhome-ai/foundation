use app_facade_api::{
    integration_presentation::{
        GetIntegrationPresentationInput, IntegrationPresentation, GET_TARGET,
    },
    ResponseDetail,
};
use serde_json::{json, Value};

#[test]
fn contract_schema_and_rust_agree_on_ui_metadata() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../fixtures/integration-presentation.conformance.json"
    ))
    .unwrap();
    let schema: Value = serde_json::from_str(include_str!(
        "../schema/integration-presentation.v1.schema.json"
    ))
    .unwrap();
    let capability: Value = serde_json::from_str(include_str!(
        "../schema/capability-descriptor.v1.schema.json"
    ))
    .unwrap();
    let validator = |part: &str| {
        jsonschema::options()
            .with_resource(
                schema["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(schema.clone()).unwrap(),
            )
            .with_resource(
                capability["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(capability.clone()).unwrap(),
            )
            .build(&json!({"$ref": format!("{}#/$defs/{part}",schema["$id"].as_str().unwrap())}))
            .unwrap()
    };
    let input_schema = validator("input");
    for (key, valid) in [("valid", true), ("invalid", false)] {
        for input in fixtures["input"][key].as_array().unwrap() {
            assert_eq!(input_schema.is_valid(input), valid, "{input}");
            assert_eq!(
                serde_json::from_value::<GetIntegrationPresentationInput>(input.clone())
                    .map(|v| v.validate().is_ok())
                    .unwrap_or(false),
                valid
            );
        }
    }
    let data = fixtures["output"]["valid"][0].clone();
    assert!(validator("output").is_valid(&data));
    for invalid in fixtures["output"]["invalid"].as_array().unwrap() {
        assert!(!validator("output").is_valid(invalid));
        assert!(serde_json::from_value::<IntegrationPresentation>(invalid.clone()).is_err());
    }
    let typed: IntegrationPresentation = serde_json::from_value(data.clone()).unwrap();
    for capability in &typed.capabilities {
        capability.validate().unwrap();
    }
    assert_eq!(serde_json::to_value(typed).unwrap(), data);
    let full = serde_json::to_value(
        app_facade_api::response::present(GET_TARGET, data.clone(), ResponseDetail::Full).unwrap(),
    )
    .unwrap();
    let essential = serde_json::to_value(
        app_facade_api::response::present(GET_TARGET, data.clone(), ResponseDetail::Essential)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(full["data"], data);
    assert_eq!(
        serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
        data
    );
    assert_eq!(
        app_facade_api::authorization::required(GET_TARGET),
        Some(app_facade_api::authorization::OperationPolicy::SpaceRead)
    );
    assert!(app_facade_api::authorization::required("/integration/meta/get").is_none());
    assert_eq!(
        app_facade_api::routing::route_policy_for_target(GET_TARGET)
            .unwrap()
            .execution,
        app_facade_api::routing::ExecutionSelector::ScopeMode
    );
}
