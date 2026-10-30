use app_facade_api::hub::HubSetupStartResponse;
use serde_json::json;
#[test]
fn setup_results_distinguish_pairing_from_committed_progress() {
    for outcome in ["RUNNING", "COMPLETED"] {
        let value = json!({"scopeId":"space","hubId":"hub","outcome":outcome});
        let parsed: HubSetupStartResponse = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), value);
        let mut invalid = value;
        invalid["pairingCode"] = json!("123456");
        assert!(serde_json::from_value::<HubSetupStartResponse>(invalid).is_err());
    }
    assert!(serde_json::from_value::<HubSetupStartResponse>(
        json!({"outcome":"PAIRING_REQUIRED","scopeId":"space"})
    )
    .is_err());
}
