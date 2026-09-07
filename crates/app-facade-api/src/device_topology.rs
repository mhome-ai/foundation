//! Public Device Integration relationship graph.
//!
//! The graph describes independently identified entities and typed relations.
//! Edge direction is semantic; it does not imply containment, a single parent,
//! a fixed number of layers, or an acyclic tree.

use crate::topology::{TopologyDurability, TopologyObservation, TopologyRuntime};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

pub const CONTRACT: &str = "mhome.device.topology.v1";
pub const GET_TARGET: &str = "/app/device/topology/get";
pub const CHANGED_TARGET: &str = "/app/device/topology/changed";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceTopologyGetRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceTopologySnapshot {
    pub contract: String,
    pub scope_id: String,
    pub generation: String,
    pub revision: u64,
    pub observed_at_ms: i64,
    pub completeness: DeviceTopologyCompleteness,
    #[serde(default)]
    pub issues: Vec<DeviceTopologyIssue>,
    #[serde(default)]
    pub entities: Vec<DeviceTopologyEntity>,
    #[serde(default)]
    pub edges: Vec<DeviceTopologyEdge>,
}

impl DeviceTopologySnapshot {
    /// Validate graph-wide invariants that JSON Schema cannot express.
    ///
    /// This deliberately does not impose parent cardinality, connectivity,
    /// acyclicity, or any fixed path through the entity kinds.
    pub fn validate(&self) -> Result<(), DeviceTopologyValidationError> {
        if self.contract != CONTRACT {
            return Err(DeviceTopologyValidationError::InvalidContract);
        }
        if self.completeness == DeviceTopologyCompleteness::Complete && !self.issues.is_empty() {
            return Err(DeviceTopologyValidationError::CompleteWithIssues);
        }
        if self.completeness == DeviceTopologyCompleteness::Partial && self.issues.is_empty() {
            return Err(DeviceTopologyValidationError::PartialWithoutIssues);
        }

        let mut entities_by_id = HashMap::new();
        let mut root_count = 0;
        for entity in &self.entities {
            let id = entity.id();
            if entities_by_id.insert(id, entity).is_some() {
                return Err(DeviceTopologyValidationError::DuplicateEntityId(
                    id.to_string(),
                ));
            }
            if matches!(entity, DeviceTopologyEntity::Root { .. }) {
                root_count += 1;
            }
        }
        match root_count {
            0 => return Err(DeviceTopologyValidationError::MissingRoot),
            1 => {}
            _ => return Err(DeviceTopologyValidationError::MultipleRoots),
        }

        for issue in &self.issues {
            if let Some(entity_id) = issue.entity_id.as_deref() {
                if !entities_by_id.contains_key(entity_id) {
                    return Err(DeviceTopologyValidationError::DanglingIssueEntity(
                        entity_id.to_string(),
                    ));
                }
            }
        }

        let mut edge_ids = HashSet::new();
        for edge in &self.edges {
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(DeviceTopologyValidationError::DuplicateEdgeId(
                    edge.id.clone(),
                ));
            }
            if edge.source == edge.target {
                return Err(DeviceTopologyValidationError::SelfEdge(edge.id.clone()));
            }
            let source = entities_by_id.get(edge.source.as_str()).ok_or_else(|| {
                DeviceTopologyValidationError::DanglingEdge {
                    edge_id: edge.id.clone(),
                    endpoint: edge.source.clone(),
                }
            })?;
            let target = entities_by_id.get(edge.target.as_str()).ok_or_else(|| {
                DeviceTopologyValidationError::DanglingEdge {
                    edge_id: edge.id.clone(),
                    endpoint: edge.target.clone(),
                }
            })?;
            if !edge.relation.accepts(source, target) {
                return Err(DeviceTopologyValidationError::InvalidRelationEndpoints {
                    edge_id: edge.id.clone(),
                    relation: edge.relation,
                    source_kind: source.kind(),
                    target_kind: target.kind(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceTopologyCompleteness {
    Complete,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceTopologyIssue {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum DeviceTopologyEntity {
    Root {
        id: String,
        integration_id: String,
        display_name: String,
    },
    Provider {
        id: String,
        integration_id: String,
        display_name: String,
        mode: String,
        multi_connect: bool,
    },
    Connection {
        id: String,
        integration_id: String,
        connection_id: String,
        display_name: String,
        runtime: TopologyRuntime,
    },
    PluginInstance {
        id: String,
        node_id: String,
        node_type: String,
        host_id: String,
        service_instance_id: String,
        display_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        host_display_name: Option<String>,
        runtime: TopologyRuntime,
    },
    SourceDevice {
        id: String,
        source_type: String,
        source_id: String,
        display_name: String,
        role: DeviceTopologySourceRole,
        runtime: TopologyRuntime,
    },
    Device {
        id: String,
        entity_id: String,
        integration_id: String,
        display_name: String,
        enabled: bool,
        runtime: TopologyRuntime,
    },
}

impl DeviceTopologyEntity {
    pub fn id(&self) -> &str {
        match self {
            Self::Root { id, .. }
            | Self::Provider { id, .. }
            | Self::Connection { id, .. }
            | Self::PluginInstance { id, .. }
            | Self::SourceDevice { id, .. }
            | Self::Device { id, .. } => id,
        }
    }

    pub fn kind(&self) -> DeviceTopologyEntityKind {
        match self {
            Self::Root { .. } => DeviceTopologyEntityKind::Root,
            Self::Provider { .. } => DeviceTopologyEntityKind::Provider,
            Self::Connection { .. } => DeviceTopologyEntityKind::Connection,
            Self::PluginInstance { .. } => DeviceTopologyEntityKind::PluginInstance,
            Self::SourceDevice { .. } => DeviceTopologyEntityKind::SourceDevice,
            Self::Device { .. } => DeviceTopologyEntityKind::Device,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceTopologyEntityKind {
    Root,
    Provider,
    Connection,
    PluginInstance,
    SourceDevice,
    Device,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceTopologySourceRole {
    Bridge,
    Device,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceTopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: DeviceTopologyRelation,
    pub basis: DeviceTopologyRelationBasis,
    pub durability: TopologyDurability,
    pub observation: TopologyObservation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DeviceTopologyRelation {
    RootProvider,
    PluginProvider,
    PluginConnection,
    ProviderConnection,
    ProviderDevice,
    ConnectionDevice,
    ConnectionSource,
    SourceDevice,
}

impl DeviceTopologyRelation {
    fn accepts(self, source: &DeviceTopologyEntity, target: &DeviceTopologyEntity) -> bool {
        use DeviceTopologyEntityKind as Kind;
        matches!(
            (self, source.kind(), target.kind()),
            (Self::RootProvider, Kind::Root, Kind::Provider)
                | (Self::PluginProvider, Kind::PluginInstance, Kind::Provider)
                | (
                    Self::PluginConnection,
                    Kind::PluginInstance,
                    Kind::Connection
                )
                | (Self::ProviderConnection, Kind::Provider, Kind::Connection)
                | (Self::ProviderDevice, Kind::Provider, Kind::Device)
                | (Self::ConnectionDevice, Kind::Connection, Kind::Device)
                | (Self::ConnectionSource, Kind::Connection, Kind::SourceDevice)
                | (Self::SourceDevice, Kind::SourceDevice, Kind::Device)
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceTopologyRelationBasis {
    IntegrationInstall,
    PluginBinding,
    ConnectionOwnership,
    TwinSource,
    SourceProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceTopologyChanged {
    pub scope_id: String,
    pub generation: String,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceTopologyValidationError {
    InvalidContract,
    CompleteWithIssues,
    PartialWithoutIssues,
    MissingRoot,
    MultipleRoots,
    DanglingIssueEntity(String),
    DuplicateEntityId(String),
    DuplicateEdgeId(String),
    DanglingEdge {
        edge_id: String,
        endpoint: String,
    },
    SelfEdge(String),
    InvalidRelationEndpoints {
        edge_id: String,
        relation: DeviceTopologyRelation,
        source_kind: DeviceTopologyEntityKind,
        target_kind: DeviceTopologyEntityKind,
    },
}

impl fmt::Display for DeviceTopologyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract => formatter.write_str("invalid Device Topology contract"),
            Self::CompleteWithIssues => {
                formatter.write_str("a complete Device Topology snapshot cannot contain issues")
            }
            Self::PartialWithoutIssues => {
                formatter.write_str("a partial Device Topology snapshot must explain its issues")
            }
            Self::MissingRoot => formatter.write_str("Device Topology snapshot has no root"),
            Self::MultipleRoots => {
                formatter.write_str("Device Topology snapshot has more than one root")
            }
            Self::DanglingIssueEntity(id) => {
                write!(formatter, "Device Topology issue references missing entity {id}")
            }
            Self::DuplicateEntityId(id) => write!(formatter, "duplicate entity id: {id}"),
            Self::DuplicateEdgeId(id) => write!(formatter, "duplicate edge id: {id}"),
            Self::DanglingEdge { edge_id, endpoint } => {
                write!(formatter, "edge {edge_id} references missing entity {endpoint}")
            }
            Self::SelfEdge(id) => write!(formatter, "edge {id} references the same entity twice"),
            Self::InvalidRelationEndpoints {
                edge_id,
                relation,
                source_kind,
                target_kind,
            } => write!(
                formatter,
                "edge {edge_id} has invalid endpoints for {relation:?}: {source_kind:?} -> {target_kind:?}"
            ),
        }
    }
}

impl std::error::Error for DeviceTopologyValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{TopologyObservationState, TopologyRuntimeState};
    use serde_json::json;

    fn runtime() -> TopologyRuntime {
        TopologyRuntime {
            state: TopologyRuntimeState::Healthy,
            reason: None,
            updated_at_ms: Some(1),
        }
    }

    fn observation() -> TopologyObservation {
        TopologyObservation {
            state: TopologyObservationState::Connected,
            observed_at_ms: 1,
            reason: None,
        }
    }

    fn entity_id(entity: &DeviceTopologyEntity) -> String {
        entity.id().to_string()
    }

    fn graph() -> DeviceTopologySnapshot {
        let root = DeviceTopologyEntity::Root {
            id: "root:basic.device".to_string(),
            integration_id: "basic.device".to_string(),
            display_name: "Devices".to_string(),
        };
        let provider = DeviceTopologyEntity::Provider {
            id: "provider:matter".to_string(),
            integration_id: "mlinkMatter.device".to_string(),
            display_name: "Matter".to_string(),
            mode: "local".to_string(),
            multi_connect: false,
        };
        let plugin = DeviceTopologyEntity::PluginInstance {
            id: "plugin:matter-1".to_string(),
            node_id: "matter-1".to_string(),
            node_type: "matter".to_string(),
            host_id: "host-1".to_string(),
            service_instance_id: "service-1".to_string(),
            display_name: "Matter Service".to_string(),
            host_display_name: Some("Living Room Mac".to_string()),
            runtime: runtime(),
        };
        let connection = DeviceTopologyEntity::Connection {
            id: "connection:matter-1".to_string(),
            integration_id: "mlinkMatter.device".to_string(),
            connection_id: "matter-1".to_string(),
            display_name: "Living Room Mac".to_string(),
            runtime: runtime(),
        };
        let device = DeviceTopologyEntity::Device {
            id: "device:light-1".to_string(),
            entity_id: "light-1".to_string(),
            integration_id: "mlinkMatter.device".to_string(),
            display_name: "Desk Light".to_string(),
            enabled: true,
            runtime: runtime(),
        };
        let edges = vec![
            DeviceTopologyEdge {
                id: "root-provider".to_string(),
                source: entity_id(&root),
                target: entity_id(&provider),
                relation: DeviceTopologyRelation::RootProvider,
                basis: DeviceTopologyRelationBasis::IntegrationInstall,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
            DeviceTopologyEdge {
                id: "plugin-provider".to_string(),
                source: entity_id(&plugin),
                target: entity_id(&provider),
                relation: DeviceTopologyRelation::PluginProvider,
                basis: DeviceTopologyRelationBasis::PluginBinding,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
            DeviceTopologyEdge {
                id: "plugin-connection".to_string(),
                source: entity_id(&plugin),
                target: entity_id(&connection),
                relation: DeviceTopologyRelation::PluginConnection,
                basis: DeviceTopologyRelationBasis::PluginBinding,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
            DeviceTopologyEdge {
                id: "provider-connection".to_string(),
                source: entity_id(&provider),
                target: entity_id(&connection),
                relation: DeviceTopologyRelation::ProviderConnection,
                basis: DeviceTopologyRelationBasis::ConnectionOwnership,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
            DeviceTopologyEdge {
                id: "provider-device".to_string(),
                source: entity_id(&provider),
                target: entity_id(&device),
                relation: DeviceTopologyRelation::ProviderDevice,
                basis: DeviceTopologyRelationBasis::TwinSource,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
            DeviceTopologyEdge {
                id: "connection-device".to_string(),
                source: entity_id(&connection),
                target: entity_id(&device),
                relation: DeviceTopologyRelation::ConnectionDevice,
                basis: DeviceTopologyRelationBasis::TwinSource,
                durability: TopologyDurability::Durable,
                observation: observation(),
            },
        ];
        DeviceTopologySnapshot {
            contract: CONTRACT.to_string(),
            scope_id: "space-1".to_string(),
            generation: "generation-1".to_string(),
            revision: 1,
            observed_at_ms: 1,
            completeness: DeviceTopologyCompleteness::Complete,
            issues: Vec::new(),
            entities: vec![root, provider, plugin, connection, device],
            edges,
        }
    }

    #[test]
    fn wire_names_are_stable() {
        let entity = DeviceTopologyEntity::PluginInstance {
            id: "plugin:camera-1".to_string(),
            node_id: "camera-1".to_string(),
            node_type: "camera".to_string(),
            host_id: "host-1".to_string(),
            service_instance_id: "service-1".to_string(),
            display_name: "Camera Service".to_string(),
            host_display_name: None,
            runtime: runtime(),
        };
        let value = serde_json::to_value(entity).unwrap();
        assert_eq!(value["kind"], "pluginInstance");
        assert_eq!(value["serviceInstanceId"], "service-1");
        assert!(value.get("hostDisplayName").is_none());

        let edge = serde_json::to_value(DeviceTopologyEdge {
            id: "plugin:camera-1->connection:camera-1".to_string(),
            source: "plugin:camera-1".to_string(),
            target: "connection:camera-1".to_string(),
            relation: DeviceTopologyRelation::PluginConnection,
            basis: DeviceTopologyRelationBasis::PluginBinding,
            durability: TopologyDurability::Durable,
            observation: observation(),
        })
        .unwrap();
        assert_eq!(edge["relation"], "plugin-connection");
        assert_eq!(edge["basis"], "pluginBinding");
    }

    #[test]
    fn graph_allows_multiple_relations_to_one_device() {
        graph().validate().unwrap();
    }

    #[test]
    fn provider_can_supply_a_device_without_a_connection() {
        let mut snapshot = graph();
        snapshot.entities.retain(|entity| {
            !matches!(
                entity,
                DeviceTopologyEntity::Connection { .. }
                    | DeviceTopologyEntity::PluginInstance { .. }
            )
        });
        snapshot.edges.retain(|edge| {
            matches!(
                edge.relation,
                DeviceTopologyRelation::RootProvider | DeviceTopologyRelation::ProviderDevice
            )
        });
        snapshot.validate().unwrap();
    }

    #[test]
    fn validation_rejects_graph_integrity_errors() {
        let mut duplicate = graph();
        duplicate.entities.push(duplicate.entities[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(DeviceTopologyValidationError::DuplicateEntityId(_))
        ));

        let mut dangling = graph();
        dangling.edges[0].target = "provider:missing".to_string();
        assert!(matches!(
            dangling.validate(),
            Err(DeviceTopologyValidationError::DanglingEdge { .. })
        ));

        let mut invalid_relation = graph();
        invalid_relation.edges[0].target = "device:light-1".to_string();
        assert!(matches!(
            invalid_relation.validate(),
            Err(DeviceTopologyValidationError::InvalidRelationEndpoints { .. })
        ));
    }

    #[test]
    fn complete_snapshot_cannot_report_partial_failures() {
        let mut snapshot = graph();
        snapshot.issues.push(DeviceTopologyIssue {
            code: "matterUnavailable".to_string(),
            message: "Matter device details are temporarily unavailable".to_string(),
            entity_id: None,
        });
        assert_eq!(
            snapshot.validate(),
            Err(DeviceTopologyValidationError::CompleteWithIssues)
        );

        let mut unexplained = graph();
        unexplained.completeness = DeviceTopologyCompleteness::Partial;
        assert_eq!(
            unexplained.validate(),
            Err(DeviceTopologyValidationError::PartialWithoutIssues)
        );
    }

    #[test]
    fn strict_objects_reject_unknown_fields() {
        assert!(serde_json::from_value::<DeviceTopologySnapshot>(json!({
            "contract": CONTRACT,
            "scopeId": "space-1",
            "generation": "generation-1",
            "revision": 1,
            "observedAtMs": 1,
            "completeness": "complete",
            "issues": [],
            "entities": [],
            "edges": [],
            "unexpected": true
        }))
        .is_err());

        assert!(serde_json::from_value::<DeviceTopologyEntity>(json!({
            "kind": "root",
            "id": "root:basic.device",
            "integrationId": "basic.device",
            "displayName": "Devices",
            "unexpected": true
        }))
        .is_err());
    }
}
