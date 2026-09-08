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
    pub issues: Vec<DeviceTopologyIssue>,
    pub entities: Vec<DeviceTopologyEntity>,
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
        require_non_empty(&self.scope_id, "scopeId")?;
        require_non_empty(&self.generation, "generation")?;
        require_non_negative(self.observed_at_ms, "observedAtMs")?;
        if self.completeness == DeviceTopologyCompleteness::Complete && !self.issues.is_empty() {
            return Err(DeviceTopologyValidationError::CompleteWithIssues);
        }
        if self.completeness == DeviceTopologyCompleteness::Partial && self.issues.is_empty() {
            return Err(DeviceTopologyValidationError::PartialWithoutIssues);
        }

        let mut entities_by_id = HashMap::new();
        let mut device_integration_count = 0;
        for (index, entity) in self.entities.iter().enumerate() {
            entity.validate_fields(index)?;
            let id = entity.id();
            if entities_by_id.insert(id, entity).is_some() {
                return Err(DeviceTopologyValidationError::DuplicateEntityId(
                    id.to_string(),
                ));
            }
            if matches!(entity, DeviceTopologyEntity::DeviceIntegration { .. }) {
                device_integration_count += 1;
            }
        }
        match device_integration_count {
            0 => return Err(DeviceTopologyValidationError::MissingDeviceIntegration),
            1 => {}
            _ => return Err(DeviceTopologyValidationError::MultipleDeviceIntegrations),
        }

        for (index, issue) in self.issues.iter().enumerate() {
            require_non_empty(&issue.code, format!("issues[{index}].code"))?;
            require_non_empty(&issue.message, format!("issues[{index}].message"))?;
            if let Some(entity_id) = issue.entity_id.as_deref() {
                require_non_empty(entity_id, format!("issues[{index}].entityId"))?;
                if !entities_by_id.contains_key(entity_id) {
                    return Err(DeviceTopologyValidationError::DanglingIssueEntity(
                        entity_id.to_string(),
                    ));
                }
            }
        }

        let mut edge_ids = HashSet::new();
        for (index, edge) in self.edges.iter().enumerate() {
            require_non_empty(&edge.id, format!("edges[{index}].id"))?;
            require_non_empty(&edge.source, format!("edges[{index}].source"))?;
            require_non_empty(&edge.target, format!("edges[{index}].target"))?;
            validate_observation(&edge.observation, format!("edges[{index}].observation"))?;
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
            if !edge.relation.accepts_basis(edge.basis) {
                return Err(DeviceTopologyValidationError::InvalidRelationBasis {
                    edge_id: edge.id.clone(),
                    relation: edge.relation,
                    basis: edge.basis,
                });
            }
            if !edge.relation.accepts_identity(source, target) {
                return Err(DeviceTopologyValidationError::InvalidRelationIdentity {
                    edge_id: edge.id.clone(),
                    relation: edge.relation,
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
    DeviceIntegration {
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
            Self::DeviceIntegration { id, .. }
            | Self::Provider { id, .. }
            | Self::Connection { id, .. }
            | Self::PluginInstance { id, .. }
            | Self::SourceDevice { id, .. }
            | Self::Device { id, .. } => id,
        }
    }

    pub fn kind(&self) -> DeviceTopologyEntityKind {
        match self {
            Self::DeviceIntegration { .. } => DeviceTopologyEntityKind::DeviceIntegration,
            Self::Provider { .. } => DeviceTopologyEntityKind::Provider,
            Self::Connection { .. } => DeviceTopologyEntityKind::Connection,
            Self::PluginInstance { .. } => DeviceTopologyEntityKind::PluginInstance,
            Self::SourceDevice { .. } => DeviceTopologyEntityKind::SourceDevice,
            Self::Device { .. } => DeviceTopologyEntityKind::Device,
        }
    }

    fn validate_fields(&self, index: usize) -> Result<(), DeviceTopologyValidationError> {
        let prefix = format!("entities[{index}]");
        require_non_empty(self.id(), format!("{prefix}.id"))?;
        match self {
            Self::DeviceIntegration {
                integration_id,
                display_name,
                ..
            } => {
                require_non_empty(integration_id, format!("{prefix}.integrationId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
            }
            Self::Provider {
                integration_id,
                display_name,
                mode,
                ..
            } => {
                require_non_empty(integration_id, format!("{prefix}.integrationId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
                require_non_empty(mode, format!("{prefix}.mode"))?;
            }
            Self::Connection {
                integration_id,
                connection_id,
                display_name,
                runtime,
                ..
            } => {
                require_non_empty(integration_id, format!("{prefix}.integrationId"))?;
                require_non_empty(connection_id, format!("{prefix}.connectionId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
                validate_runtime(runtime, format!("{prefix}.runtime"))?;
            }
            Self::PluginInstance {
                node_id,
                node_type,
                host_id,
                service_instance_id,
                display_name,
                host_display_name,
                runtime,
                ..
            } => {
                require_non_empty(node_id, format!("{prefix}.nodeId"))?;
                require_non_empty(node_type, format!("{prefix}.nodeType"))?;
                require_non_empty(host_id, format!("{prefix}.hostId"))?;
                require_non_empty(service_instance_id, format!("{prefix}.serviceInstanceId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
                if let Some(host_display_name) = host_display_name {
                    require_non_empty(host_display_name, format!("{prefix}.hostDisplayName"))?;
                }
                validate_runtime(runtime, format!("{prefix}.runtime"))?;
            }
            Self::SourceDevice {
                source_type,
                source_id,
                display_name,
                runtime,
                ..
            } => {
                require_non_empty(source_type, format!("{prefix}.sourceType"))?;
                require_non_empty(source_id, format!("{prefix}.sourceId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
                validate_runtime(runtime, format!("{prefix}.runtime"))?;
            }
            Self::Device {
                entity_id,
                integration_id,
                display_name,
                runtime,
                ..
            } => {
                require_non_empty(entity_id, format!("{prefix}.entityId"))?;
                require_non_empty(integration_id, format!("{prefix}.integrationId"))?;
                require_non_empty(display_name, format!("{prefix}.displayName"))?;
                validate_runtime(runtime, format!("{prefix}.runtime"))?;
            }
        }
        Ok(())
    }
}

fn require_non_empty(
    value: &str,
    path: impl Into<String>,
) -> Result<(), DeviceTopologyValidationError> {
    if value.is_empty() {
        return Err(DeviceTopologyValidationError::InvalidField {
            path: path.into(),
            requirement: "must not be empty",
        });
    }
    Ok(())
}

fn require_non_negative(
    value: i64,
    path: impl Into<String>,
) -> Result<(), DeviceTopologyValidationError> {
    if value < 0 {
        return Err(DeviceTopologyValidationError::InvalidField {
            path: path.into(),
            requirement: "must be non-negative",
        });
    }
    Ok(())
}

fn validate_runtime(
    runtime: &TopologyRuntime,
    path: impl Into<String>,
) -> Result<(), DeviceTopologyValidationError> {
    if let Some(updated_at_ms) = runtime.updated_at_ms {
        require_non_negative(updated_at_ms, format!("{}.updatedAtMs", path.into()))?;
    }
    Ok(())
}

fn validate_observation(
    observation: &TopologyObservation,
    path: impl Into<String>,
) -> Result<(), DeviceTopologyValidationError> {
    require_non_negative(
        observation.observed_at_ms,
        format!("{}.observedAtMs", path.into()),
    )
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeviceTopologyEntityKind {
    DeviceIntegration,
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
    IntegrationProvider,
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
            (
                Self::IntegrationProvider,
                Kind::DeviceIntegration,
                Kind::Provider
            ) | (Self::PluginProvider, Kind::PluginInstance, Kind::Provider)
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

    fn accepts_basis(self, basis: DeviceTopologyRelationBasis) -> bool {
        matches!(
            (self, basis),
            (
                Self::IntegrationProvider,
                DeviceTopologyRelationBasis::IntegrationInstall
            ) | (
                Self::PluginProvider | Self::PluginConnection,
                DeviceTopologyRelationBasis::PluginBinding
            ) | (
                Self::ProviderConnection,
                DeviceTopologyRelationBasis::ConnectionOwnership
            ) | (
                Self::ProviderDevice | Self::ConnectionDevice,
                DeviceTopologyRelationBasis::TwinSource
            ) | (
                Self::ConnectionSource | Self::SourceDevice,
                DeviceTopologyRelationBasis::SourceProjection
            )
        )
    }

    fn accepts_identity(
        self,
        source: &DeviceTopologyEntity,
        target: &DeviceTopologyEntity,
    ) -> bool {
        match (self, source, target) {
            (
                Self::ProviderConnection,
                DeviceTopologyEntity::Provider {
                    integration_id: provider_integration,
                    ..
                },
                DeviceTopologyEntity::Connection {
                    integration_id: connection_integration,
                    ..
                },
            ) => provider_integration == connection_integration,
            (
                Self::ProviderDevice,
                DeviceTopologyEntity::Provider {
                    integration_id: provider_integration,
                    ..
                },
                DeviceTopologyEntity::Device {
                    integration_id: device_integration,
                    ..
                },
            ) => provider_integration == device_integration,
            (
                Self::ConnectionDevice,
                DeviceTopologyEntity::Connection {
                    integration_id: connection_integration,
                    ..
                },
                DeviceTopologyEntity::Device {
                    integration_id: device_integration,
                    ..
                },
            ) => connection_integration == device_integration,
            _ => true,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceTopologyValidationError {
    InvalidContract,
    InvalidField {
        path: String,
        requirement: &'static str,
    },
    CompleteWithIssues,
    PartialWithoutIssues,
    MissingDeviceIntegration,
    MultipleDeviceIntegrations,
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
    InvalidRelationBasis {
        edge_id: String,
        relation: DeviceTopologyRelation,
        basis: DeviceTopologyRelationBasis,
    },
    InvalidRelationIdentity {
        edge_id: String,
        relation: DeviceTopologyRelation,
    },
}

impl fmt::Display for DeviceTopologyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract => formatter.write_str("invalid Device Topology contract"),
            Self::InvalidField { path, requirement } => {
                write!(formatter, "invalid {path}: {requirement}")
            }
            Self::CompleteWithIssues => {
                formatter.write_str("a complete Device Topology snapshot cannot contain issues")
            }
            Self::PartialWithoutIssues => {
                formatter.write_str("a partial Device Topology snapshot must explain its issues")
            }
            Self::MissingDeviceIntegration => {
                formatter.write_str("Device Topology snapshot has no Device Integration entity")
            }
            Self::MultipleDeviceIntegrations => {
                formatter.write_str("Device Topology snapshot has more than one Device Integration entity")
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
            Self::InvalidRelationBasis {
                edge_id,
                relation,
                basis,
            } => write!(
                formatter,
                "edge {edge_id} has invalid basis {basis:?} for {relation:?}"
            ),
            Self::InvalidRelationIdentity { edge_id, relation } => write!(
                formatter,
                "edge {edge_id} connects inconsistent integration identities for {relation:?}"
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
        let device_integration = DeviceTopologyEntity::DeviceIntegration {
            id: "integration:basic.device".to_string(),
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
                id: "integration-provider".to_string(),
                source: entity_id(&device_integration),
                target: entity_id(&provider),
                relation: DeviceTopologyRelation::IntegrationProvider,
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
            entities: vec![device_integration, provider, plugin, connection, device],
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
                DeviceTopologyRelation::IntegrationProvider
                    | DeviceTopologyRelation::ProviderDevice
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

        let mut invalid_basis = graph();
        invalid_basis.edges[0].basis = DeviceTopologyRelationBasis::TwinSource;
        assert!(matches!(
            invalid_basis.validate(),
            Err(DeviceTopologyValidationError::InvalidRelationBasis { .. })
        ));

        let mut invalid_identity = graph();
        let DeviceTopologyEntity::Connection { integration_id, .. } =
            &mut invalid_identity.entities[3]
        else {
            panic!("fixture must contain the Connection entity");
        };
        *integration_id = "other.device".to_string();
        assert!(matches!(
            invalid_identity.validate(),
            Err(DeviceTopologyValidationError::InvalidRelationIdentity { .. })
        ));
    }

    #[test]
    fn validation_matches_schema_field_constraints() {
        let mut empty_scope = graph();
        empty_scope.scope_id.clear();
        assert_eq!(
            empty_scope.validate(),
            Err(DeviceTopologyValidationError::InvalidField {
                path: "scopeId".to_string(),
                requirement: "must not be empty",
            })
        );

        let mut empty_display_name = graph();
        let DeviceTopologyEntity::DeviceIntegration { display_name, .. } =
            &mut empty_display_name.entities[0]
        else {
            panic!("fixture must begin with the Device Integration entity");
        };
        display_name.clear();
        assert_eq!(
            empty_display_name.validate(),
            Err(DeviceTopologyValidationError::InvalidField {
                path: "entities[0].displayName".to_string(),
                requirement: "must not be empty",
            })
        );

        let mut negative_runtime_time = graph();
        let DeviceTopologyEntity::PluginInstance { runtime, .. } =
            &mut negative_runtime_time.entities[2]
        else {
            panic!("fixture must contain the Plugin Instance entity");
        };
        runtime.updated_at_ms = Some(-1);
        assert_eq!(
            negative_runtime_time.validate(),
            Err(DeviceTopologyValidationError::InvalidField {
                path: "entities[2].runtime.updatedAtMs".to_string(),
                requirement: "must be non-negative",
            })
        );

        let mut negative_observation_time = graph();
        negative_observation_time.edges[0]
            .observation
            .observed_at_ms = -1;
        assert_eq!(
            negative_observation_time.validate(),
            Err(DeviceTopologyValidationError::InvalidField {
                path: "edges[0].observation.observedAtMs".to_string(),
                requirement: "must be non-negative",
            })
        );

        let mut empty_provider_mode = graph();
        let DeviceTopologyEntity::Provider { mode, .. } = &mut empty_provider_mode.entities[1]
        else {
            panic!("fixture must contain the Provider entity");
        };
        mode.clear();
        assert_eq!(
            empty_provider_mode.validate(),
            Err(DeviceTopologyValidationError::InvalidField {
                path: "entities[1].mode".to_string(),
                requirement: "must not be empty",
            })
        );
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
            "kind": "deviceIntegration",
            "id": "integration:basic.device",
            "integrationId": "basic.device",
            "displayName": "Devices",
            "unexpected": true
        }))
        .is_err());

        assert!(serde_json::from_value::<DeviceTopologyGetRequest>(json!({
            "unexpected": true
        }))
        .is_err());

    }
}
