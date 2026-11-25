use app_facade_api::{authorization, response, routing, ResponseDetail};
use serde_json::{json, Value};

const TARGET: &str = "/app/dashboard/card/authoring/get";

fn validator(definition: &str) -> jsonschema::Validator {
    let mut schema: Value = serde_json::from_str(include_str!(
        "../schema/dashboard-card-authoring.v1.schema.json"
    ))
    .unwrap();
    schema["$ref"] = json!(format!("#/definitions/{definition}"));
    jsonschema::validator_for(&schema).unwrap()
}

#[test]
fn authoring_read_is_space_scoped_and_cannot_select_other_context() {
    assert_eq!(
        authorization::required(TARGET),
        Some(authorization::OperationPolicy::SpaceRead)
    );
    assert_eq!(
        routing::route_policy_for_target(TARGET).unwrap().execution,
        routing::ExecutionSelector::SpaceMode
    );
    let input = validator("input");
    assert!(input.is_valid(&json!({"cardId":"c1"})));
    for invalid in [
        json!({}),
        json!({"cardId":" "}),
        json!({"cardId":null}),
        json!({"cardId":"x".repeat(257)}),
        json!({"cardId":"c1","spaceId":"other"}),
        json!({"cardId":"c1","includeValues":true}),
    ] {
        assert!(!input.is_valid(&invalid), "{invalid}");
    }
}

#[test]
fn authoring_response_preserves_patterns_in_both_presentations_and_rejects_resource_data() {
    let output = validator("output");
    for pattern in [
        Some("basic.device.onoff_brightness_color"),
        Some("default.basic.weather"),
        Some("automation"),
        None,
    ] {
        let data = json!({"spaceId":"s1","cardId":"c1","canvasId":"canvas1",
            "title":"Living room","primaryViewId":null,
            "views":[{"id":null,"name":null,"patternId":pattern}]});
        assert!(output.is_valid(&data));
        let full = response::present(TARGET, data.clone(), ResponseDetail::Full).unwrap();
        let essential = response::present(TARGET, data.clone(), ResponseDetail::Essential).unwrap();
        assert_eq!(serde_json::to_value(full).unwrap()["data"], data);
        let essential = serde_json::to_value(essential).unwrap();
        assert_eq!(
            serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
            data
        );
        for key in ["entity", "widgets", "layouts", "resources"] {
            let mut invalid = data.clone();
            invalid[key] = json!({});
            assert!(!output.is_valid(&invalid));
        }
        let mut invalid = data;
        invalid["views"][0]["source"] = json!({"baseId":"private"});
        assert!(!output.is_valid(&invalid));
    }
}
