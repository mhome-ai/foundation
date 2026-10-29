use app_facade_api::group::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
fn roundtrip<T: DeserializeOwned + Serialize>(value: Value) -> Value {
    serde_json::to_value(serde_json::from_value::<T>(value).unwrap()).unwrap()
}
#[test]
fn group_dtos_roundtrip_shared_contracts() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/group.v1.schema.json")).unwrap();
    let corpus: Value =
        serde_json::from_str(include_str!("../fixtures/group.conformance.json")).unwrap();
    for (kind, valid) in [("valid", true), ("invalid", false)] {
        for item in corpus[kind].as_array().unwrap() {
            let name = item["definition"].as_str().unwrap();
            let mut root = schema.clone();
            root["$ref"] = format!("#/$defs/{name}").into();
            assert_eq!(
                jsonschema::validator_for(&root)
                    .unwrap()
                    .is_valid(&item["value"]),
                valid,
                "{name}"
            );
            if !valid {
                continue;
            }
            let value = item["value"].clone();
            let actual = match name {
                "ListGroupsInput" => roundtrip::<ListGroupsInput>(value),
                "GetGroupsInput" => roundtrip::<GetGroupsInput>(value),
                "CreateGroupInput" => roundtrip::<CreateGroupInput>(value),
                "RenameGroupInput" => roundtrip::<RenameGroupInput>(value),
                "UpdateGroupMembersInput" => roundtrip::<UpdateGroupMembersInput>(value),
                "DeleteGroupInput" => roundtrip::<DeleteGroupInput>(value),
                "GroupSummary" => roundtrip::<GroupSummary>(value),
                "GroupDetail" => roundtrip::<GroupDetail>(value),
                "GroupMutationOutput" => roundtrip::<GroupMutationOutput>(value),
                "DeleteGroupOutput" => roundtrip::<DeleteGroupOutput>(value),
                "ListGroupsOutput" => roundtrip::<ListGroupsOutput>(value),
                "GetGroupsOutput" => roundtrip::<GetGroupsOutput>(value),
                _ => panic!("Unknown fixture {name}"),
            };
            assert_eq!(actual, item["value"], "{name}");
        }
    }
}

#[test]
fn partial_member_resolution_keeps_definition_and_standard_warning() {
    let data = serde_json::json!({"spaceId":"s", "items":[{"requestedId":"g:1", "found":true,
        "group":{"groupId":"g:1","entityType":"basic.device.Device","entityIds":["d1"],"revision":3}}]});
    let response =
        serde_json::to_value(app_facade_api::response::outcome(DETAIL, data.clone())).unwrap();
    assert_eq!(response["status"], "partial");
    assert_eq!(response["data"], data);
    assert_eq!(response["warnings"][0]["details"]["groupId"], "g:1");
    assert_eq!(response["warnings"][0]["code"], "GROUP_MEMBERS_UNAVAILABLE");
    let mut complete = data;
    complete["items"][0]["group"]["capabilities"] = serde_json::json!([]);
    complete["items"][0]["group"]["memberProblems"] = serde_json::json!([]);
    assert_eq!(
        serde_json::to_value(app_facade_api::response::outcome(DETAIL, complete)).unwrap()
            ["status"],
        "success"
    );
}
