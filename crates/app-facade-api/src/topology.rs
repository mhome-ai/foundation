use serde::{Deserialize, Serialize};

pub const CONTRACT: &str = "mhome.space.topology.v1";
pub const GET_TARGET: &str = "/app/topology/get";
pub const CHANGED_TARGET: &str = "/app/topology/changed";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TopologyGetRequest {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopologySnapshot {
    pub contract: String,
    pub scope_id: String,
    pub generation: String,
    pub revision: u64,
    pub observed_at_ms: i64,
    #[serde(default)]
    pub entities: Vec<TopologyEntity>,
    #[serde(default)]
    pub edges: Vec<TopologyEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TopologyEntity {
    Host {
        id: String,
        host_id: String,
        display_name: String,
    },
    Hub {
        id: String,
        hub_id: String,
        host_id: String,
        display_name: String,
    },
    Node {
        id: String,
        node_id: String,
        node_type: String,
        host_id: String,
        display_name: String,
        binding_state: String,
        runtime: TopologyRuntime,
    },
    Client {
        id: String,
        app_client_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cur_client_id: Option<String>,
        source: String,
        device_type: String,
        display_name: String,
        is_current: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        route_kind: Option<TopologyRouteKind>,
        has_local_credential: bool,
    },
    Cloud {
        id: String,
        display_name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopologyRuntime {
    pub state: TopologyRuntimeState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TopologyRuntimeState {
    Healthy,
    Degraded,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TopologyRouteKind {
    LocalDirect,
    CloudBridgeShadow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopologyEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: TopologyRelation,
    pub basis: TopologyRelationBasis,
    pub durability: TopologyDurability,
    pub observation: TopologyObservation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum TopologyRelation {
    HubCloud,
    NodeHub,
    ClientHub,
    ClientCloud,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TopologyRelationBasis {
    HubCommission,
    NodeBinding,
    LocalCredential,
    LiveRoute,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TopologyDurability {
    Durable,
    Ephemeral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopologyObservation {
    pub state: TopologyObservationState,
    pub observed_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TopologyObservationState {
    Connected,
    Disconnected,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TopologyChanged {
    pub scope_id: String,
    pub generation: String,
    pub revision: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_contract_uses_stable_wire_names() {
        let entity = TopologyEntity::Client {
            id: "client:a:desktop".to_string(),
            app_client_id: "a:desktop".to_string(),
            cur_client_id: Some("L:socket".to_string()),
            source: "desktop".to_string(),
            device_type: "desktop".to_string(),
            display_name: "Desktop".to_string(),
            is_current: true,
            route_kind: Some(TopologyRouteKind::LocalDirect),
            has_local_credential: true,
        };
        let value = serde_json::to_value(entity).unwrap();
        assert_eq!(value["kind"], "client");
        assert_eq!(value["routeKind"], "localDirect");

        let edge = serde_json::to_value(TopologyEdge {
            id: "client:a:desktop->hub:h".to_string(),
            source: "client:a:desktop".to_string(),
            target: "hub:h".to_string(),
            relation: TopologyRelation::ClientHub,
            basis: TopologyRelationBasis::LocalCredential,
            durability: TopologyDurability::Durable,
            observation: TopologyObservation {
                state: TopologyObservationState::Disconnected,
                observed_at_ms: 1,
                reason: None,
            },
        })
        .unwrap();
        assert_eq!(edge["relation"], "client-hub");
        assert_eq!(edge["basis"], "localCredential");
        assert_eq!(edge["observation"]["state"], "disconnected");
    }
}
