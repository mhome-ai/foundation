//! Resource reads separate definitions, current observations and relationships.
//! UI rendering never requires loading an execution program or a resource's history.
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const ENTITY_STATE: &str = "/app/entity/state/get";
pub const DEVICE_STATE: &str = "/app/device/state/get";
pub const DEVICE_SERIES: &str = "/app/presentation/device/history/series/get";
pub const WIDGET_VALUE: &str = "/app/widget/value/get";
pub const AUTOMATION_STATUSES: &str = "/app/automation/status";
pub const GRAPH_RELATIONS: &str = "/app/graph/relations/get";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIdsInput {
    pub ids: Vec<String>,
}

impl ResourceIdsInput {
    pub fn normalized_ids(&self) -> Result<Vec<String>, &'static str> {
        if self.ids.is_empty() || self.ids.len() > 100 {
            return Err("ids must contain 1 to 100 resource IDs");
        }
        let mut ids = Vec::new();
        for id in &self.ids {
            let id = id.trim();
            if id.is_empty() {
                return Err("ids must not contain blank values");
            }
            if !ids.iter().any(|item| item == id) {
                ids.push(id.to_string());
            }
        }
        Ok(ids)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceReadItem<T> {
    pub requested_id: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<T>,
}
impl<T> ResourceReadItem<T> {
    pub fn new(id: String, resource: Option<T>) -> Self {
        Self {
            requested_id: id,
            found: resource.is_some(),
            reason: resource.is_none().then(|| "not_found".into()),
            resource,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReadOutput<T> {
    pub items: Vec<ResourceReadItem<T>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityObservation {
    pub capability_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EntityStateInput {
    pub entity_type: String,
    pub ids: Vec<String>,
}
impl EntityStateInput {
    pub fn normalized_ids(&self) -> Result<Vec<String>, &'static str> {
        ResourceIdsInput {
            ids: self.ids.clone(),
        }
        .normalized_ids()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityObservation {
    pub entity_id: String,
    pub latest: Vec<CapabilityObservation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceObservation {
    pub entity_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub online_updated_at_sec: Option<i64>,
    pub latest: Vec<CapabilityObservation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WidgetValue {
    pub widget_id: String,
    pub revision: i64,
    pub value: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_timestamp: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_reads_are_bounded_and_ordered() {
        let input = ResourceIdsInput {
            ids: vec![" b ".into(), "a".into(), "b".into()],
        };
        assert_eq!(input.normalized_ids().unwrap(), ["b", "a"]);
        for ids in [vec![], vec![" ".into()], vec!["a".into(); 101]] {
            assert!(ResourceIdsInput { ids }.normalized_ids().is_err());
        }
        assert!(
            serde_json::from_str::<ResourceIdsInput>(r#"{"ids":["a"],"refresh":true}"#).is_err()
        );
    }
    #[test]
    fn entity_observations_require_typed_bounded_ids_without_runtime_options() {
        for ids in [vec![], vec![" ".into()], vec!["x".into(); 101]] {
            assert!(EntityStateInput {
                entity_type: "Location".into(),
                ids
            }
            .normalized_ids()
            .is_err());
        }
        assert_eq!(
            EntityStateInput {
                entity_type: "Location".into(),
                ids: vec![" home ".into(), "home".into()]
            }
            .normalized_ids()
            .unwrap(),
            ["home"]
        );
        assert!(serde_json::from_value::<EntityStateInput>(
            serde_json::json!({"entityType":"Location","ids":["home"],"refresh":true})
        )
        .is_err());
        assert!(
            serde_json::from_value::<EntityStateInput>(serde_json::json!({"ids":["home"]}))
                .is_err()
        );
    }
    #[test]
    fn missing_and_empty_observations_are_different() {
        let item = ResourceReadItem::new(
            "a".into(),
            Some(DeviceObservation {
                entity_id: "a".into(),
                online: None,
                online_updated_at_sec: None,
                latest: vec![],
            }),
        );
        let value = serde_json::to_value(item).unwrap();
        assert_eq!(value["found"], true);
        assert!(value["resource"].get("online").is_none());
        assert_eq!(value["resource"]["latest"], serde_json::json!([]));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSeriesInput {
    pub device_id: String,
    #[serde(default)]
    pub capability_ids: Vec<String>,
}

/// Relationships are directional and one-hop; no arbitrary graph traversal.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRef {
    pub kind: ResourceKind,
    pub id: String,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    Device,
    Entity,
    Automation,
    Widget,
    Card,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GraphRelationsInput {
    pub resource: ResourceRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRelation {
    pub from: ResourceRef,
    pub relation: String,
    pub to: ResourceRef,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capability_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphRelationsOutput {
    pub resource: ResourceRef,
    pub relations: Vec<ResourceRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceViewInput {
    pub device_id: String,
}

/// Onboarding maps Provider-local identities into stable current-Space Device IDs.
/// This read never starts synchronization and never chooses an ambiguous match.
pub const DEVICE_SOURCE_RESOLVE: &str = "/app/device/source/resolve";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceSourceResolveInput {
    pub integration_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connect_id: Option<String>,
    pub source_ids: Vec<String>,
}
impl DeviceSourceResolveInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.integration_id.trim().is_empty()
            || self
                .connect_id
                .as_ref()
                .is_some_and(|s| s.trim().is_empty())
        {
            return Err("integrationId and any supplied connectId must not be blank");
        }
        if self.source_ids.is_empty()
            || self.source_ids.len() > 100
            || self.source_ids.iter().any(|s| s.trim().is_empty())
        {
            return Err("sourceIds must contain 1 to 100 nonblank Provider-local IDs");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSourceResolveItem {
    pub requested_source_id: String,
    pub found: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceSourceResolveOutput {
    pub items: Vec<DeviceSourceResolveItem>,
}

#[cfg(test)]
mod device_source_tests {
    use super::*;
    #[test]
    fn source_ids_are_opaque_bounded_and_strict() {
        let input: DeviceSourceResolveInput = serde_json::from_value(serde_json::json!({"integrationId":"provider","connectId":"home","sourceIds":["  opaque  ", "\"quoted\""]})).unwrap();
        assert!(input.validate().is_ok());
        assert_eq!(input.source_ids[0], "  opaque  ");
        for ids in [vec![], vec![" ".into()], vec!["a".into(); 101]] {
            assert!(DeviceSourceResolveInput {
                integration_id: "provider".into(),
                connect_id: None,
                source_ids: ids
            }
            .validate()
            .is_err());
        }
        assert!(serde_json::from_value::<DeviceSourceResolveInput>(
            serde_json::json!({"integrationId":"provider","sourceIds":["a"],"refresh":true})
        )
        .is_err());
    }
}
