// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Value};

#[test]
fn layout_shape_is_one_level_and_requires_all_profiles() {
    let schema: Value =
        serde_json::from_str(include_str!("../schema/canvas-layout.v1.schema.json")).unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let rect = json!({"x":0,"y":0,"w":1,"h":1});
    let profiles = json!({"sm":rect,"md":rect,"lg":rect});
    let member = json!({"cardId":"lamp","layouts":profiles});
    let mut document = json!({"schemaVersion":1,"items":[{"kind":"composition","id":"group","layouts":profiles,"members":[member,{"cardId":"motion","layouts":profiles}]}]});
    assert!(validator.is_valid(&document));
    document["items"][0]["members"][0]["members"] = json!([]);
    assert!(!validator.is_valid(&document));
    document["items"][0]["members"][0]
        .as_object_mut()
        .unwrap()
        .remove("members");
    document["items"][0]["layouts"]
        .as_object_mut()
        .unwrap()
        .remove("sm");
    assert!(!validator.is_valid(&document));
}
#[test]
fn every_composition_command_requires_space_write_authority() {
    for target in [
        "/app/dashboard/canvas/composition/create",
        "/app/dashboard/canvas/composition/ungroup",
        "/app/dashboard/canvas/composition/layout/update",
    ] {
        assert_eq!(
            app_facade_api::authorization::required(target),
            Some(app_facade_api::authorization::OperationPolicy::SpaceWrite),
            "{target}"
        );
    }
}
#[test]
fn old_whole_card_skin_api_is_not_a_content_renderer() {
    let package: Value =
        serde_json::from_str(include_str!("../schema/card-skin-package.v1.schema.json")).unwrap();
    assert_eq!(package["properties"]["apiVersion"]["const"], json!(2));
    assert_eq!(
        package["definitions"]["project"]["properties"]["apiVersion"]["const"],
        json!(2)
    );
}

#[test]
fn commands_accept_multiple_profiles_and_reject_ambiguous_drafts() {
    let command = include_str!("../schema/dashboard-composition.v1.schema.json").replace(
        "canvas-layout.v1.schema.json#/definitions/rect",
        "#/definitions/rect",
    );
    let mut schema: Value = serde_json::from_str(&command).unwrap();
    let layout: Value =
        serde_json::from_str(include_str!("../schema/canvas-layout.v1.schema.json")).unwrap();
    schema["definitions"]["rect"] = layout["definitions"]["rect"].clone();
    for definition in ["create", "ungroup", "updateMembers"] {
        schema["$ref"] = json!(format!("#/definitions/{definition}"));
        let validator = jsonschema::validator_for(&schema).unwrap();
        let draft = json!({"layoutSize":"sm","items":[]});
        let mut input = json!({"canvasId":"canvas","expectedRevision":1,"rootDrafts":[draft,{"layoutSize":"md","items":[]}]});
        match definition {
            "create" => input["itemIds"] = json!(["a", "b"]),
            "ungroup" => input["compositionId"] = json!("group"),
            _ => {
                input["compositionId"] = json!("group");
                input["layoutSize"] = json!("sm");
                input["reset"] = json!(true);
            }
        }
        assert!(validator.is_valid(&input), "{definition}");
        input["rootDraft"] = draft.clone();
        assert!(!validator.is_valid(&input));
        input.as_object_mut().unwrap().remove("rootDrafts");
        assert!(validator.is_valid(&input));
        input.as_object_mut().unwrap().remove("rootDraft");
        input["rootDrafts"] = json!([draft, draft, draft, draft]);
        assert!(!validator.is_valid(&input));
    }
}
