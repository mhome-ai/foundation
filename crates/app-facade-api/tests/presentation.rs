use app_facade_api::{response, ResponseDetail};
use serde_json::{json, Value};

#[test]
fn space_member_changes_validate_inputs_and_complete_result_dtos() {
    let schema: Value = serde_json::from_str(include_str!(
        "../schema/space-member-changes.v1.schema.json"
    ))
    .unwrap();
    let validator = |definition: &str| {
        let mut root = schema.clone();
        root["$ref"] = json!(format!("#/definitions/{definition}"));
        jsonschema::validator_for(&root).unwrap()
    };
    let input = validator("input");
    assert!(input.is_valid(&json!({"removeEmails":["a@example.com"],"inviteEmails":[]})));
    assert!(input.is_valid(&json!({"inviteEmails":["b@example.com"]})));
    for invalid in [
        json!({}),
        json!({"removeEmails":[],"inviteEmails":[]}),
        json!({"removeEmails":[false]}),
        json!({"inviteEmails":["a"],"mode":"direct"}),
    ] {
        assert!(!input.is_valid(&invalid));
    }
    let output = validator("output");
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../fixtures/presentation/cases.json")).unwrap();
    for case in cases
        .iter()
        .filter(|case| case["target"] == "/app/space/member/update")
    {
        output.validate(&case["data"]).unwrap();
    }
}

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
            response::present_in_timezone(
                target,
                case["data"].clone(),
                ResponseDetail::Essential,
                case["timezone"].as_str().unwrap_or("UTC"),
            )
            .unwrap(),
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
            assert_eq!(full["data"], *case.get("full").unwrap_or(&case["data"]));
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

#[test]
fn plugin_business_fields_do_not_determine_facade_outcome() {
    for plugin in ["camera", "example"] {
        let target = format!("/app/plugin/{plugin}/device/list");
        let data = serde_json::json!({"nodeId":"node", "nodeType":plugin, "version":"v1", "data":{"ok":false,"reason":"resource state"}});
        let response = serde_json::to_value(
            response::present(&target, data.clone(), ResponseDetail::Full).unwrap(),
        )
        .unwrap();
        assert_eq!(response["status"], "success");
        assert_eq!(response["data"], data);
        let failure =
            app_facade_api::FacadeResponse::error("PRECONDITION_FAIL", "Resource changed");
        let response =
            serde_json::to_value(response::render(&target, failure, ResponseDetail::Full).unwrap())
                .unwrap();
        assert_eq!(response["status"], "error");
        assert_eq!(response["error"]["code"], "PRECONDITION_FAIL");
    }
}

#[test]
fn card_skin_edits_require_an_explicit_ui_base() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/card-skins.v1.schema.json")).unwrap();
    for (definition, input) in [
        (
            "install",
            serde_json::json!({"baseUi":"default-pod","packageId":"cards.pod","version":"1.0.0","expectedRevision":null}),
        ),
        (
            "overrideSet",
            serde_json::json!({"baseUi":"default-pod","patternId":"basic.device.onOff","installationId":"installed","rendererId":"switch","config":{},"expectedRevision":"revision"}),
        ),
        (
            "overrideReset",
            serde_json::json!({"baseUi":"default-pod","patternId":"basic.device.onOff","expectedRevision":"revision"}),
        ),
    ] {
        let entry = serde_json::json!({"$ref":format!("{}#/definitions/{definition}",schema["$id"].as_str().unwrap())});
        let package: Value =
            serde_json::from_str(include_str!("../schema/card-skin-package.v1.schema.json"))
                .unwrap();
        let validator = jsonschema::options()
            .with_resource(
                package["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(package.clone()).unwrap(),
            )
            .with_resource(
                schema["$id"].as_str().unwrap(),
                jsonschema::Resource::from_contents(schema.clone()).unwrap(),
            )
            .build(&entry)
            .unwrap();
        validator.validate(&input).unwrap();
        let mut absent = input.clone();
        absent.as_object_mut().unwrap().remove("baseUi");
        assert!(
            !validator.is_valid(&absent),
            "{definition} must not infer Default"
        );
        let mut invalid = input.clone();
        invalid["baseUi"] = serde_json::json!("default_pod");
        assert!(!validator.is_valid(&invalid));
    }
}

#[test]
fn unset_widget_values_and_history_survive_final_warning_rendering() {
    use serde_json::json;
    for (target, data) in [
        (
            "/app/widget/value/get",
            json!({"items":[{"requestedId":"value-d","found":true,"resource":{"widgetId":"value-d","revision":1,"value":null}}]}),
        ),
        (
            "/app/widget/history/list",
            json!({"history":[{"timestamp":1767225600,"value":null}]}),
        ),
        (
            "/app/widget/value/get",
            json!({"items":[{"requestedId":"value-d","found":true,"resource":{"widgetId":"value-d","revision":1,"value":{"custom":null}}}]}),
        ),
    ] {
        let settled = app_facade_api::FacadeResponse::success(data.clone()).with_warning(
            app_facade_api::FacadeProblem {
                code: "OBSERVATION".into(),
                message: "Read completed".into(),
                details: None,
            },
        );
        let essential =
            response::render(target, settled.clone(), ResponseDetail::Essential).unwrap();
        let full = response::render(target, settled.clone(), ResponseDetail::Full).unwrap();
        assert_eq!(full, settled);
        let envelope = serde_json::to_value(essential).unwrap();
        let visible: Value =
            serde_yaml::from_str(&response::tool_text(&envelope, ResponseDetail::Essential))
                .unwrap();
        let mut expected = data;
        response::time::localize(target, &mut expected, "UTC");
        assert_eq!(visible["result"], expected);
    }
}

#[test]
fn warning_evidence_is_displayed_without_changing_full_or_outcome() {
    let full = app_facade_api::FacadeResponse::success(json!({"saved":true})).with_warning(
        app_facade_api::FacadeProblem {
            code: "OBSERVATION".into(),
            message: "Retained evidence".into(),
            details: Some(json!({"at":{"_type":"Timestamp","value":123}})),
        },
    );
    let before = serde_json::to_value(&full).unwrap();
    let rendered = serde_json::to_value(
        response::render_in_timezone(
            "/app/example",
            full.clone(),
            ResponseDetail::Essential,
            "Asia/Shanghai",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(rendered["status"], "success");
    assert_eq!(
        rendered["warnings"][0]["details"]["at"],
        "1970-01-01 08:00:00.123 +08:00 (Asia/Shanghai)"
    );
    assert_eq!(
        serde_json::to_value(response::render("/app/example", full, ResponseDetail::Full).unwrap())
            .unwrap(),
        before
    );
}
