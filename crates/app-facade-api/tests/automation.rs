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
