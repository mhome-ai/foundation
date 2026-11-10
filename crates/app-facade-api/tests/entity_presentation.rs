use app_facade_api::{
    entity_presentation::{GetEntityPresentationInput, GET_TARGET},
    ResponseDetail,
};
use serde_json::Value;

#[test]
fn selected_metadata_is_bounded_ordered_and_keeps_missing_references() {
    use app_facade_api::entity_presentation::{
        EntityMetadataOutput, GetEntityMetadataInput, META_GET_TARGET,
    };
    let fixtures: Value = serde_json::from_str(include_str!(
        "../fixtures/entity-presentation.conformance.json"
    ))
    .unwrap();
    for (kind, valid) in [("valid", true), ("invalid", false)] {
        for value in fixtures["metaInput"][kind].as_array().unwrap() {
            assert_eq!(
                serde_json::from_value::<GetEntityMetadataInput>(value.clone())
                    .map(|input| input.normalized().is_ok())
                    .unwrap_or(false),
                valid,
                "{value}"
            );
        }
    }
    let input: GetEntityMetadataInput =
        serde_json::from_value(fixtures["metaInput"]["valid"][0].clone()).unwrap();
    assert_eq!(input.normalized().unwrap().1, ["lamp", "g:room", "missing"]);
    let data = fixtures["metaOutput"]["valid"][0].clone();
    let typed: EntityMetadataOutput = serde_json::from_value(data.clone()).unwrap();
    assert_eq!(serde_json::to_value(typed).unwrap(), data);
    assert_eq!(
        app_facade_api::authorization::required(META_GET_TARGET),
        Some(app_facade_api::authorization::OperationPolicy::SpaceRead)
    );
    assert!(app_facade_api::authorization::required("/app/entity/presentation/get").is_none());
    assert!(app_facade_api::authorization::required("/app/integration/presentation/get").is_none());
    assert_eq!(
        app_facade_api::routing::route_policy_for_target(META_GET_TARGET)
            .unwrap()
            .execution,
        app_facade_api::routing::ExecutionSelector::SpaceMode
    );
    let full = serde_json::to_value(
        app_facade_api::response::present(META_GET_TARGET, data.clone(), ResponseDetail::Full)
            .unwrap(),
    )
    .unwrap();
    let essential = serde_json::to_value(
        app_facade_api::response::present(META_GET_TARGET, data.clone(), ResponseDetail::Essential)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(full["data"], data);
    assert_eq!(
        serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
        data
    );
}

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
    let projected = data;
    assert!(projected["entityCard"].is_null());
    assert_eq!(
        serde_yaml::from_str::<Value>(essential["data"].as_str().unwrap()).unwrap(),
        projected
    );
}
