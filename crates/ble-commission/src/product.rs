//! A paired product binds once, then reuses standard Host and Hub authorization.
use crate::{cloud, device::Client, platform::Cloud};
use app_facade_api::pod::{PodInfo, POD_CREDENTIAL_ENDPOINT};
use core_api::host::access::*;
use host_protocol::{access, signatures::*};
use serde_json::{json, Value};
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) async fn prepare(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    scope: &str,
    info: &PodInfo,
    client: &Arc<Client>,
    first_hub: bool,
    network_host: Option<&(String, core_api::host::auth::PublicKey)>,
) -> Result<Option<app_facade_api::pod::HubIdentity>, String> {
    let binding = info
        .device_binding
        .as_ref()
        .ok_or("Product binding missing")?;
    binding.validate().map_err(str::to_owned)?;
    if binding.client.id != info.device_id || binding.client.key_id != info.identity.key_id {
        return Err("Product binding belongs to another Client".into());
    }
    let secret = random_id();
    let probe = random_id();
    let hash = access::exchange_digest(&secret).map_err(|e| e.to_string())?;
    let response = client
        .call_json(
            POD_CREDENTIAL_ENDPOINT,
            &json!({"op":"prepareBinding","request":{"probeId":probe,"exchangeKeyHash":hash}}),
        )
        .await
        .map_err(|e| e.to_string())?;
    let compact = text(&response, "proof")?;
    let challenge: HostChallenge =
        serde_json::from_value(jws_payload_json(compact).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let key = verifying_key(&challenge.host_public_key).map_err(|e| e.to_string())?;
    let challenge: HostChallenge =
        verify_jws(&key, CHALLENGE_TYPE, compact).map_err(|e| e.to_string())?;
    access::validate_challenge(
        &challenge,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    )
    .map_err(|e| e.to_string())?;
    if challenge.status.mode != HostMode::SpaceDep
        || challenge.device_binding.as_ref() != Some(binding)
        || challenge.probe_id != probe
        || challenge.exchange_key_hash.as_deref() != Some(&hash)
    {
        return Err("Host proof does not match the paired product".into());
    }
    if network_host.is_some_and(|(id, key)| {
        id != &challenge.status.host_id || key != &challenge.host_public_key
    }) {
        return Err("The product Host changed after Wi-Fi setup".into());
    }
    // Fresh physical pairing plus this explicit owner approval confirms the
    // current product identity after reset, using the normal directory CAS.
    let issued: Value = cloud::post(
        cloud,
        context,
        "/host/authorizations/claim",
        None,
        &json!({"challenge":compact,"spaceId":scope,"recoverIdentity":true}),
    )
    .await
    .map_err(|e| e.to_string())?;
    let proof = text(&issued, "proof")?;
    let result = client
        .call_json(
            POD_CREDENTIAL_ENDPOINT,
            &json!({"op":"bind","request":{
        "env":binding.env,"proof":proof,"exchangeKey":secret,"claimToken":null}}),
        )
        .await
        .map_err(|e| e.to_string())?;
    let receipt = text(&result, "proof")?;
    let verified: AuthorizationReceipt =
        verify_jws(&key, RECEIPT_TYPE, receipt).map_err(|e| e.to_string())?;
    if verified.status.mode != HostMode::SpaceDep
        || verified.status.env != binding.env
        || verified.status.host_key_id != challenge.status.host_key_id
        || verified.proof != proof
        || verified.status.host_id != challenge.status.host_id
        || verified.status.generation != binding.generation
        || !matches!(&verified.status.claim, Some(Claim::Space {space}) if space.validate().is_ok() && space.space_id == scope)
    {
        return Err("Product did not confirm the requested Space".into());
    }
    let _: Value = cloud::post(
        cloud,
        context,
        "/host/authorizations/commit-claim",
        None,
        &json!({"claimId":text(&issued,"claimId")?,"proof":receipt}),
    )
    .await
    .map_err(|e| e.to_string())?;
    if !first_hub {
        // An ordinary product joins the selected Hub; it never promotes its
        // bundled Core or invokes a Hub mutation.
        return Ok(None);
    }
    cloud
        .setup_product_hub(
            context,
            scope,
            &challenge.status.host_id,
            &serde_json::to_string(&challenge.host_public_key).map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
    let hub = cloud::resolve_setup_hub(cloud, context, scope)
        .await
        .map_err(|e| e.to_string())?
        .ok_or("The first Hub was not published")?;
    if hub.host_id != challenge.status.host_id || hub.host_public_key != challenge.host_public_key {
        return Err("The first Hub does not belong to the paired product".into());
    }
    Ok(Some(hub))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value[key]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("Product authorization missing {key}"))
}
