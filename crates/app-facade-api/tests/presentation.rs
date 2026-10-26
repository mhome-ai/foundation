use app_facade_api::{ResponseDetail, response};
use serde_json::Value;
#[test]
fn shared_projection_and_outcome_contract() {
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
        assert_eq!(full["status"], case["status"], "{}", case["name"]);
        assert_eq!(essential["status"], case["status"]);
        if case["status"] == "error" {
            assert_eq!(full["error"]["code"], case["code"]);
            assert_eq!(essential, full);
        } else {
            assert_eq!(full["data"], case["data"]);
            assert!(essential.get("data").is_none());
            let parsed: Value = serde_yaml::from_str(essential["text"].as_str().unwrap()).unwrap();
            assert_eq!(parsed, case["essential"], "{}", case["name"]);
        }
    }
}
