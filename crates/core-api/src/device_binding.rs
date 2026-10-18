//! Durable identity of a fixed-Space product. Business credentials remain private
//! to its Client. A dependent Host reads this binding, never a second claim.
use crate::{runtime_space::SpaceIdentity, RuntimeEnv};
use serde::{Deserialize, Serialize};

pub const DEVICE_BINDING_VERSION: u32 = 1;
pub const DEVICE_BINDING_PATH: &str = "security/device-binding.json";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceComponentIdentity {
    pub id: String,
    pub key_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceBinding {
    pub version: u32,
    pub env: RuntimeEnv,
    pub client: DeviceComponentIdentity,
    pub host: Option<DeviceComponentIdentity>,
    /// Replaced only by factory reset. Credentials and grants must match it.
    pub generation: String,
    /// None means initialized locally but not yet bound by the owner.
    pub space: Option<SpaceIdentity>,
}

impl DeviceComponentIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if valid_id(&self.id) && valid_id(&self.key_id) {
            Ok(())
        } else {
            Err("invalid device component identity")
        }
    }
}
impl DeviceBinding {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != DEVICE_BINDING_VERSION || !valid_id(&self.generation) {
            return Err("invalid device binding version or generation");
        }
        self.client.validate()?;
        if let Some(host) = &self.host {
            host.validate()?;
        }
        if let Some(space) = &self.space {
            space
                .validate()
                .map_err(|_| "invalid device Space binding")?;
        }
        Ok(())
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 256 && id.trim() == id
}
