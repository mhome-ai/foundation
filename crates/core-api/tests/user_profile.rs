use core_api::UserInfo;

#[test]
fn user_identity_round_trips_without_retired_privilege_settings() {
    let user: UserInfo = serde_json::from_value(serde_json::json!({
        "id": "user", "name": "Ada", "email": "ada@example.test",
        "setting": {"debugEnabled": true, "scriptMode": true},
        "accountClass": "A", "eng": true
    }))
    .unwrap();
    assert_eq!(
        serde_json::to_value(user).unwrap(),
        serde_json::json!({
            "id": "user", "name": "Ada", "email": "ada@example.test"
        })
    );
}
