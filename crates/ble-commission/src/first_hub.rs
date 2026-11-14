//! Cloud-owned first-Hub intent and short-lived network recovery permit.
use crate::{cloud, platform::Cloud};
use core_api::{host::auth::PublicKey, runtime_space::SpaceIdentity};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Intent {
    pub intent_id: String,
    pub space: SpaceIdentity,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Permit {
    intent_id: String,
    space: SpaceIdentity,
    authorization_id: String,
}
impl Intent {
    pub fn from_target(target: &Value) -> Result<Self, String> {
        let intent = Self {
            intent_id: target["intentId"].as_str().unwrap_or_default().into(),
            space: SpaceIdentity {
                space_id: target["spaceId"].as_str().unwrap_or_default().into(),
                tenant_id: target["tenantId"].as_str().unwrap_or_default().into(),
            },
        };
        intent.validate()?;
        Ok(intent)
    }
    fn validate(&self) -> Result<(), String> {
        self.space.validate().map_err(str::to_owned)?;
        if !host_protocol::access::valid_secret(&self.intent_id) {
            return Err("Invalid first-Hub setup intent".into());
        }
        Ok(())
    }
}
pub(crate) async fn prepare(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    space_id: &str,
    host_id: &str,
    host_key: &PublicKey,
    recovery: Option<&Intent>,
) -> Result<Value, String> {
    if let Some(intent) = recovery {
        let permit: Permit = cloud::post(
            cloud,
            context,
            "/host/first-hub/recovery/prepare",
            None,
            &json!({"spaceId":space_id,"intentId":intent.intent_id}),
        )
        .await
        .map_err(|e| e.to_string())?;
        if permit.intent_id != intent.intent_id
            || permit.space != intent.space
            || permit.space.space_id != space_id
            || !host_protocol::access::valid_secret(&permit.authorization_id)
        {
            return Err("Network recovery authorization does not match this product".into());
        }
        Ok(
            json!({"op":"prepareWifi","mode":"bootstrapRecovery","firstHub":intent,
            "authorizationId":permit.authorization_id}),
        )
    } else {
        let intent: Intent = cloud::post(cloud, context, "/host/first-hub/begin", None,
            &json!({"spaceId":space_id,"hostId":host_id,"env":core_api::RuntimeEnv::current().as_str(),
                "hostPublicKey":host_key})).await.map_err(|e|e.to_string())?;
        intent.validate()?;
        if intent.space.space_id != space_id {
            return Err("First-Hub setup belongs to another Space".into());
        }
        Ok(json!({"op":"prepareWifi","mode":"bootstrap","firstHub":intent}))
    }
}
