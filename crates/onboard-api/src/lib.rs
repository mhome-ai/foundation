//! Canonical product onboarding contract. No transports, account sessions,
//! Integration models or product workflows belong in this crate.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_ID_BYTES: usize = 64;
pub const MAX_SAFE_TIMESTAMP: u64 = 9_007_199_254_740_991;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Host,
    Pod,
    Wave,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Environment {
    Prod,
    Dev,
    E2e,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Ownership {
    Space,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Identity {
    pub id: String,
    pub key_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub role: Role,
    pub identity: Identity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NetworkOwner {
    External,
    Product,
    None,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Network {
    pub owner: NetworkOwner,
    pub lan: bool,
    pub ble: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Product {
    pub protocol_version: u32,
    pub ownership: Ownership,
    pub environment: Environment,
    pub identity: Identity,
    /// Random 128-bit installation generation, replaced only by product reset.
    pub generation: String,
    pub components: Vec<Component>,
    pub network: Network,
    pub can_host_hub: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Space {
    pub tenant_id: String,
    pub space_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "intent", rename_all = "camelCase", deny_unknown_fields)]
pub enum Hub {
    Join {
        #[serde(rename = "hubId")]
        hub_id: String,
        #[serde(rename = "keyId")]
        key_id: String,
    },
    First,
}

/// Safe to persist: contains no PIN, Wi-Fi password, user token, component
/// credential or authorization proof. An operation ID is never a bearer token.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub protocol_version: u32,
    pub operation_id: String,
    pub generation: String,
    pub descriptor_digest: String,
    pub space: Space,
    pub hub: Hub,
}

/// The verifier must validate signature, issuer, expiry, purpose and proof of
/// possession against the exact Product and Plan before starting device work.
/// Intentionally has no Debug implementation: the proof is an authorization secret.
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Start {
    pub plan: Plan,
    pub authorization_proof: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Network,
    Binding,
    Host,
    Hub,
    Pod,
    Wave,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Running,
    NeedsInput,
    NeedsAuthorization,
    RecoveryRequired,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComponentState {
    Pending,
    Confirmed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ComponentResult {
    pub role: Role,
    pub state: ComponentState,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Status {
    pub plan: Plan,
    pub phase: Phase,
    pub state: State,
    pub components: Vec<ComponentResult>,
    pub reason: Option<String>,
    pub updated_at_ms: u64,
}

fn id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ID_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b':'))
}
fn hex(value: &str, len: usize) -> bool {
    value.len() == len
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn key(value: &str) -> bool {
    value.strip_prefix("p256:").is_some_and(|v| hex(v, 64))
}

impl Identity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if id(&self.id) && key(&self.key_id) {
            Ok(())
        } else {
            Err("invalid product/component identity")
        }
    }
}

impl Product {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.protocol_version != PROTOCOL_VERSION || !hex(&self.generation, 32) {
            return Err("unsupported protocol or invalid installation generation");
        }
        self.identity.validate()?;
        let mut roles = BTreeSet::new();
        if self.components.is_empty() || self.components.len() > 3 {
            return Err("invalid component set");
        }
        for component in &self.components {
            component.identity.validate()?;
            if !roles.insert(component.role) {
                return Err("duplicate component role");
            }
        }
        if self.can_host_hub && !roles.contains(&Role::Host) {
            return Err("a Hub requires a Host component");
        }
        if !self.network.lan && !self.network.ble {
            return Err("no onboarding transport");
        }
        if matches!(self.network.owner, NetworkOwner::Product) && !self.network.lan {
            return Err("managed Wi-Fi requires LAN");
        }
        if matches!(self.network.owner, NetworkOwner::None)
            && (self.network.lan || roles != BTreeSet::from([Role::Wave]))
        {
            return Err("only pure BLE Wave may have no IP network");
        }
        if !self.network.lan && !matches!(self.network.owner, NetworkOwner::None) {
            return Err("IP products require LAN");
        }
        Ok(())
    }

    /// Hash a fixed, length-delimited representation, independent of JSON field
    /// order and component ordering. Protocol v1 never hashes user secrets.
    pub fn digest(&self) -> Result<String, &'static str> {
        self.validate()?;
        let mut hash = Sha256::new();
        let mut part = |s: &str| {
            hash.update((s.len() as u32).to_be_bytes());
            hash.update(s.as_bytes());
        };
        part("meow.onboard.product.v1");
        part(match self.environment {
            Environment::Prod => "prod",
            Environment::Dev => "dev",
            Environment::E2e => "e2e",
        });
        part(&self.identity.id);
        part(&self.identity.key_id);
        part(&self.generation);
        let mut components = self.components.iter().collect::<Vec<_>>();
        components.sort_by_key(|c| c.role);
        for c in components {
            part(match c.role {
                Role::Host => "host",
                Role::Pod => "pod",
                Role::Wave => "wave",
            });
            part(&c.identity.id);
            part(&c.identity.key_id);
        }
        part(match self.network.owner {
            NetworkOwner::External => "external",
            NetworkOwner::Product => "product",
            NetworkOwner::None => "none",
        });
        part(if self.network.lan { "1" } else { "0" });
        part(if self.network.ble { "1" } else { "0" });
        part(if self.can_host_hub { "1" } else { "0" });
        Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
}

impl Plan {
    pub fn validate(&self, product: &Product) -> Result<(), &'static str> {
        product.validate()?;
        if self.protocol_version != PROTOCOL_VERSION
            || !hex(&self.operation_id, 32)
            || self.generation != product.generation
            || self.descriptor_digest != product.digest()?
            || !id(&self.space.tenant_id)
            || !id(&self.space.space_id)
        {
            return Err("invalid or substituted onboarding target");
        }
        match &self.hub {
            Hub::Join { hub_id, key_id } if id(hub_id) && key(key_id) => Ok(()),
            Hub::First if product.can_host_hub => Ok(()),
            _ => Err("invalid Hub intent or identity"),
        }
    }
}

impl Start {
    /// Structural validation only. Successful validation does not authorize work.
    pub fn validate(&self, product: &Product) -> Result<(), &'static str> {
        self.plan.validate(product)?;
        if self.authorization_proof.is_empty()
            || self.authorization_proof.len() > 8192
            || !self
                .authorization_proof
                .bytes()
                .all(|b| (0x21..=0x7e).contains(&b))
        {
            return Err("invalid authorization proof size");
        }
        Ok(())
    }
}

impl Status {
    pub fn validate(&self, product: &Product) -> Result<(), &'static str> {
        self.plan.validate(product)?;
        let expected: BTreeSet<_> = product.components.iter().map(|c| c.role).collect();
        let actual: BTreeSet<_> = self.components.iter().map(|c| c.role).collect();
        if actual != expected
            || actual.len() != self.components.len()
            || self.updated_at_ms > MAX_SAFE_TIMESTAMP
        {
            return Err("invalid operation result");
        }
        let rank = |phase| match phase {
            Phase::Network => 0,
            Phase::Binding => 1,
            Phase::Host => 2,
            Phase::Hub => 3,
            Phase::Pod => 4,
            Phase::Wave => 5,
            Phase::Complete => 6,
        };
        if matches!(self.phase, Phase::Host) && !expected.contains(&Role::Host)
            || matches!(self.phase, Phase::Pod) && !expected.contains(&Role::Pod)
            || matches!(self.phase, Phase::Wave) && !expected.contains(&Role::Wave)
            || self.components.iter().any(|c| {
                let completed = rank(self.phase)
                    > match c.role {
                        Role::Host => 2,
                        Role::Pod => 4,
                        Role::Wave => 5,
                    };
                completed != (c.state == ComponentState::Confirmed)
            })
        {
            return Err("component results disagree with the current phase");
        }
        if (self.state == State::Complete) != (self.phase == Phase::Complete)
            || (self.state == State::Complete
                && self
                    .components
                    .iter()
                    .any(|c| c.state != ComponentState::Confirmed))
        {
            return Err("incomplete product cannot report success");
        }
        if self.reason.as_ref().is_some_and(|r| !id(r))
            || (self.state != State::Running
                && self.state != State::Complete
                && self.reason.is_none())
        {
            return Err("blocked operations require a bounded reason code");
        }
        Ok(())
    }
}
