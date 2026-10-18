//! Canonical business admission. The manifest is shared by local Core and Lion.
//! Domain ownership, MCP grants, Node declarations and confirmation still apply.
use core_api::{
    client_identity::ClientPrincipal,
    space_access::{SpaceAccessFacts, SpaceAccessLevel},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::OnceLock,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationPolicy {
    UserRead,
    UserWrite,
    SpaceRead,
    SpaceWrite,
    SpaceManage,
    Node,
}

impl OperationPolicy {
    pub fn required_space_level(self) -> Option<SpaceAccessLevel> {
        match self {
            Self::UserRead | Self::UserWrite => None,
            Self::SpaceRead | Self::Node => Some(SpaceAccessLevel::Read),
            Self::SpaceWrite => Some(SpaceAccessLevel::Write),
            Self::SpaceManage => Some(SpaceAccessLevel::Manage),
        }
    }

    pub fn allows(self, facts: &SpaceAccessFacts<'_>) -> bool {
        match self.required_space_level() {
            Some(level) => facts.allows(level),
            None => {
                matches!(facts.principal, ClientPrincipal::User { user_id } if !user_id.trim().is_empty())
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AuthorizationManifest {
    pub contract_version: String,
    pub operations: BTreeMap<String, OperationPolicy>,
    pub node_prefix: String,
    pub service_operations: BTreeSet<String>,
}

pub fn manifest() -> &'static AuthorizationManifest {
    static MANIFEST: OnceLock<AuthorizationManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        serde_json::from_str(include_str!("../manifest/authorization.v1.json"))
            .expect("valid authorization manifest")
    })
}

/// Node is delegation to the current registered Node's exact operation policy,
/// never permission to execute an arbitrary extension request.
pub fn required(target: &str) -> Option<OperationPolicy> {
    let manifest = manifest();
    manifest.operations.get(target).copied().or_else(|| {
        let suffix = target.strip_prefix(&manifest.node_prefix)?;
        let (node, operation) = suffix.split_once('/')?;
        (!node.is_empty() && !operation.is_empty()).then_some(OperationPolicy::Node)
    })
}

/// Only an authenticated service ingress may use this explicit allowlist.
pub fn allows_service(target: &str) -> bool {
    manifest().service_operations.contains(target)
}
