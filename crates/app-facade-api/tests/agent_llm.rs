use app_facade_api::agent_llm::*;
use serde_json::{json, Value};

#[test]
fn shared_settings_corpus_matches_types_and_schema() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/agent-llm.v1.schema.json")).unwrap();
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/agent-llm.conformance.json")).unwrap();
    for (group, valid) in [("valid", true), ("invalid", false)] {
        for case in corpus[group].as_array().unwrap() {
            let name = case["type"].as_str().unwrap();
            let entry = json!({"$defs": schema["$defs"], "$ref": format!("#/$defs/{name}")});
            let validator = jsonschema::validator_for(&entry).unwrap();
            assert_eq!(validator.is_valid(&case["value"]), valid, "{case}");
            if valid {
                let value = case["value"].clone();
                match name {
                    "ListProvidersInput" => {
                        serde_json::from_value::<ListProvidersInput>(value).unwrap();
                    }
                    "GetProviderInput" => {
                        serde_json::from_value::<GetProviderInput>(value).unwrap();
                    }
                    "Summary" => {
                        serde_json::from_value::<Summary>(value).unwrap();
                    }
                    "ProviderList" => {
                        serde_json::from_value::<ProviderList>(value).unwrap();
                    }
                    "ProviderDetail" => {
                        serde_json::from_value::<ProviderDetail>(value).unwrap();
                    }
                    _ => panic!("unhandled fixture {name}"),
                }
            }
        }
    }
}

#[test]
fn provider_detail_preserves_full_and_projects_safe_essential_configuration() {
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/agent-llm.conformance.json")).unwrap();
    let data = corpus["valid"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["type"] == "ProviderDetail")
        .unwrap()["value"]
        .clone();
    let full = app_facade_api::response::present(
        "/app/agent/llm/provider/get",
        data.clone(),
        app_facade_api::ResponseDetail::Full,
    )
    .unwrap();
    assert_eq!(serde_json::to_value(full).unwrap()["data"], data);
    let essential = app_facade_api::response::present(
        "/app/agent/llm/provider/get",
        data,
        app_facade_api::ResponseDetail::Essential,
    )
    .unwrap();
    let essential = serde_json::to_value(essential).unwrap();
    let data: Value = serde_yaml::from_str(essential["data"].as_str().unwrap()).unwrap();
    assert_eq!(data["provider"]["configuration"]["hasCredential"], true);
    assert_eq!(data["provider"]["configuration"]["ready"], false);
    assert!(data["provider"]["configuration"].get("metadata").is_none());
    assert!(data["provider"]["configuration"].get("nodeType").is_none());
}
