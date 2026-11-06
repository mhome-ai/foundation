//! UI reference picker discovery. It does not execute the selected resource.
use serde::{Deserialize, Serialize};

pub const TYPE_LIST: &str = "/app/presentation/reference/type/list";
pub const ITEM_LIST: &str = "/app/presentation/reference/item/list";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeListInput {}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ItemListInput {
    pub entity_type: String,
}

impl ItemListInput {
    pub fn normalized_type(&self) -> Result<&str, &'static str> {
        let value = self.entity_type.trim();
        if value.is_empty() {
            return Err("entityType is required");
        }
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Device,
    Widget,
    Automation,
    Entity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceType {
    pub entity_type: String,
    pub name: String,
    pub category: Category,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceItem {
    pub id: String,
    pub name: String,
    pub entity_type: String,
    pub category: Category,
    pub capabilities: Vec<String>,
    pub group: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypeListOutput {
    pub types: Vec<ReferenceType>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemListOutput {
    pub items: Vec<ReferenceItem>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_inputs_reject_scope_overrides_and_legacy_aliases() {
        assert!(serde_json::from_value::<TypeListInput>(json!({"scopeId":"other"})).is_err());
        for input in [
            json!({}),
            json!({"type":"Device"}),
            json!({"entityType":null}),
            json!({"entityType":1}),
            json!({"entityType":"Device","all":true}),
        ] {
            assert!(serde_json::from_value::<ItemListInput>(input).is_err());
        }
        assert!(ItemListInput {
            entity_type: "  ".into()
        }
        .normalized_type()
        .is_err());
        assert_eq!(
            ItemListInput {
                entity_type: " basic.device.Device ".into()
            }
            .normalized_type()
            .unwrap(),
            "basic.device.Device"
        );
    }

    #[test]
    fn shared_outputs_preserve_identity_empty_arrays_and_false() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/reference-presentation.v1.json"))
                .unwrap();
        let types: TypeListOutput = serde_json::from_value(fixture["types"].clone()).unwrap();
        let items: ItemListOutput = serde_json::from_value(fixture["items"].clone()).unwrap();
        assert_eq!(serde_json::to_value(types).unwrap(), fixture["types"]);
        assert_eq!(serde_json::to_value(items).unwrap(), fixture["items"]);
        for target in [TYPE_LIST, ITEM_LIST] {
            assert_eq!(
                crate::authorization::required(target),
                Some(crate::authorization::OperationPolicy::SpaceRead)
            );
            assert_eq!(
                crate::routing::route_policy_for_target(target)
                    .unwrap()
                    .execution,
                crate::routing::ExecutionSelector::ScopeMode
            );
            let data = fixture[if target == TYPE_LIST {
                "types"
            } else {
                "items"
            }]
            .clone();
            assert_eq!(crate::response::essential(target, &data), data);
        }
        for target in ["/reference/entity-type/list", "/reference/entity/list"] {
            assert!(crate::authorization::required(target).is_none());
        }
    }
}
