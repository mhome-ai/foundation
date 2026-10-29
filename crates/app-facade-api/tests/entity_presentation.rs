use app_facade_api::{
    ResponseDetail,
    entity_presentation::{GET_TARGET, GetEntityPresentationInput},
};
use serde_json::Value;

#[test]
fn requests_match_the_shared_corpus_and_response_projection_preserves_ui_fields() {
    let fixtures: Value = serde_json::from_str(include_str!(
        "../fixtures/entity-presentation.conformance.json"
    ))
    .unwrap();
    for value in fixtures["input"]["valid"].as_array().unwrap() {
        serde_json::from_value::<GetEntityPresentationInput>(value.clone())
            .unwrap()
            .validate()
            .unwrap();
    }
    for value in fixtures["input"]["invalid"].as_array().unwrap() {
        assert!(
            serde_json::from_value::<GetEntityPresentationInput>(value.clone())
                .map(|input| input.validate().is_err())
                .unwrap_or(true)
        );
    }
    let data = fixtures["output"]["valid"][0].clone();
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
    assert_eq!(essential["status"], "success");
    assert_eq!(
        serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
        data
    );
}
