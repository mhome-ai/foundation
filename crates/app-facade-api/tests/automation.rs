use app_facade_api::{automation::*, response, ResponseDetail};
use serde_json::json;

#[test]
fn controls_return_state_in_both_presentations_and_reject_old_acknowledgements() {
    for target in [START, STOP] {
        let data = json!({"automationId":"a","enabled":false,"running":false});
        let full = response::present(target, data.clone(), ResponseDetail::Full).unwrap();
        assert_eq!(full.data(), Some(&data));
        let essential = response::present(target, data, ResponseDetail::Essential).unwrap();
        let text = essential.data().unwrap().as_str().unwrap();
        assert!(text.contains("running: false"));
        assert!(!response::is_acknowledgement(target));
        let invalid = response::outcome(target, json!({"success":true}));
        assert_eq!(
            serde_json::to_value(invalid).unwrap()["error"]["code"],
            "INVALID_RESPONSE"
        );
    }
}

#[test]
fn closed_inputs_separate_rename_from_description_and_identity_from_display_context() {
    serde_json::from_value::<StartInput>(
        json!({"automationId":"a","timezone":"Asia/Shanghai","locationSymbol":"Local"}),
    )
    .unwrap();
    for invalid in [
        json!({"automationId":"a","actor":"someone"}),
        json!({"automationId":"a","sideData":{}}),
    ] {
        assert!(serde_json::from_value::<StartInput>(invalid).is_err());
    }
    assert!(
        serde_json::from_value::<StopInput>(json!({"automationId":"a","timezone":"UTC"})).is_err()
    );
    assert!(serde_json::from_value::<DescriptionInput>(
        json!({"automationId":"a","expectedScriptHash":"h","name":"new"})
    )
    .is_err());
    assert!(serde_json::from_value::<RenameInput>(
        json!({"automationId":"a","name":"new","script":{}})
    )
    .is_err());
    assert_eq!(
        app_facade_api::authorization::required(RENAME),
        Some(app_facade_api::authorization::OperationPolicy::SpaceWrite)
    );
}

#[test]
fn diagnostic_metadata_and_instance_list_have_distinct_closed_input_schemas() {
    let schema: serde_json::Value =
        serde_json::from_str(include_str!("../schema/automation.v1.schema.json")).unwrap();
    let compile = |name: &str| jsonschema::validator_for(&schema["$defs"][name]).unwrap();
    let list = compile("instanceListInput");
    let get = compile("diagnosticGetInput");
    let identity = json!({"automationId":"a"});
    assert!(list.is_valid(&identity));
    assert!(get.is_valid(&identity));
    for value in [
        json!({"automationId":"a","cursor":"x"}),
        json!({"automationId":"a","limit":10}),
    ] {
        assert!(!list.is_valid(&value));
        assert!(get.is_valid(&value));
    }
    for value in [
        json!({}),
        json!({"automationId":" "}),
        json!({"automationId":"a","threadId":0}),
    ] {
        assert!(!list.is_valid(&value));
        assert!(!get.is_valid(&value));
    }
    for limit in [0, 101] {
        assert!(!get.is_valid(&json!({"automationId":"a","limit":limit})));
    }
}
