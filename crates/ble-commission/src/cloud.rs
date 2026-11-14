//! Pod credential calls made on behalf of the signed-in user.
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use app_facade_api::pod::{PodEnrollmentRequest, UserLoginApproval, USER_LOGIN_APPROVE_PATH};
use core_api::host::auth::PublicKey;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::platform::{Cloud, CloudError};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CallError {
    #[error(transparent)]
    Cloud(#[from] CloudError),
    /// Lion answered with an error envelope or a non-success status.
    #[error("{code}: {message}")]
    Api { code: String, message: String },
    #[error("unexpected response: {0}")]
    Unreadable(String),
}

pub(crate) async fn post<T: DeserializeOwned>(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    path: &str,
    space_id: Option<&str>,
    body: &impl Serialize,
) -> Result<T, CallError> {
    let body =
        serde_json::to_string(body).map_err(|error| CallError::Unreadable(error.to_string()))?;
    let response = tokio::time::timeout(
        REQUEST_TIMEOUT,
        cloud.post_json(context, path, space_id, &body),
    )
    .await
    .map_err(|_| CloudError::Unreachable(format!("{path} timed out")))??;
    let value: Value = serde_json::from_str(&response.body)
        .map_err(|error| CallError::Unreadable(format!("HTTP {}: {error}", response.status)))?;
    if let Some(code) = value.get("error").filter(|error| !error.is_null()) {
        return Err(CallError::Api {
            code: code.as_str().unwrap_or("ERROR").to_string(),
            message: value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }
    if !(200..300).contains(&response.status) {
        return Err(CallError::Api {
            code: format!("HTTP_{}", response.status),
            message: String::new(),
        });
    }
    serde_json::from_value(value).map_err(|error| CallError::Unreadable(error.to_string()))
}

pub(crate) async fn approve_login(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    request: &PodEnrollmentRequest,
) -> Result<UserLoginApproval, CallError> {
    post(cloud, context, USER_LOGIN_APPROVE_PATH, None, request).await
}

/// Selects only a published Hub. An in-flight setup or teardown is not an
/// empty Space eligible for a new bootstrap.
pub fn selected_setup_hub(
    view: &app_facade_api::hub::LocalHubState,
    space_id: &str,
    tenant_id: String,
) -> Result<Option<(String, String)>, CloudError> {
    let unavailable = || CloudError::Unreachable("The selected Space's Hub is not ready".into());
    if tenant_id.is_empty()
        || view.space_id != space_id
        || view.configured != view.hub.is_some()
        || view.operation.is_some()
        || view.preparation.is_some()
        || view.revocation.is_some()
        || view
            .pairing
            .as_ref()
            .is_some_and(|pairing| pairing.state == "RUNNING")
    {
        return Err(unavailable());
    }
    let Some(hub) = view.hub.as_ref() else {
        return Ok(None);
    };
    if !hub.cloud.status.eq_ignore_ascii_case("ready") || hub.cloud.id.is_empty() {
        return Err(unavailable());
    }
    Ok(Some((tenant_id, hub.cloud.id.clone())))
}

/// A paired product may continue only its own setup. In-flight state remains
/// unpublished until the ordinary Hub engine finishes; it is never a new slot.
pub fn selected_product_setup_hub(
    view: &app_facade_api::hub::LocalHubState,
    space_id: &str,
    tenant_id: String,
    host_id: &str,
) -> Result<Option<(String, String)>, CloudError> {
    use app_facade_api::hub::HubOperationKind;
    if host_id.is_empty()
        || view.space_id != space_id
        || tenant_id.is_empty()
        || view.configured != view.hub.is_some()
        || view.revocation.is_some()
        || view
            .preparation
            .as_ref()
            .is_some_and(|p| p.host_id() != host_id)
        || view
            .hub
            .as_ref()
            .is_some_and(|hub| hub.cloud.host_id != host_id)
        || view.operation.as_ref().is_some_and(|op| {
            op.kind != HubOperationKind::Setup || op.host_id.as_deref() != Some(host_id)
        })
        || view
            .pairing
            .as_ref()
            .is_some_and(|op| op.host_id.as_deref() != Some(host_id))
    {
        return Err(CloudError::Unreachable(
            "This Space has a Hub or operation for another device".into(),
        ));
    }
    if view.preparation.is_some()
        || view.operation.is_some()
        || view.pairing.as_ref().is_some_and(|p| p.state == "RUNNING")
    {
        return Ok(None);
    }
    selected_setup_hub(view, space_id, tenant_id)
}

pub(crate) async fn resolve_setup_hub(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    space_id: &str,
) -> Result<Option<app_facade_api::pod::HubIdentity>, CallError> {
    let Some((tenant_id, hub_id)) =
        tokio::time::timeout(REQUEST_TIMEOUT, cloud.selected_hub(context, space_id))
            .await
            .map_err(|_| CloudError::Unreachable("Hub selection timed out".into()))??
    else {
        return Ok(None);
    };
    let identity: app_facade_api::pod::HubIdentity = post(
        cloud,
        context,
        "hub/identity/get",
        None,
        &serde_json::json!({"tenantId":tenant_id,"hubId":hub_id,"spaceId":space_id}),
    )
    .await?;
    if identity.space_id != space_id || identity.hub_id != hub_id || identity.tenant_id != tenant_id
    {
        return Err(CallError::Unreadable(
            "the Hub identity does not match the selected Space".into(),
        ));
    }
    let key = host_protocol::signatures::verifying_key(&PublicKey {
        x: identity.public_key_x.clone(),
        y: identity.public_key_y.clone(),
    })
    .map_err(|_| CallError::Unreadable("invalid Hub public key".into()))?;
    let fingerprint = hex::encode(Sha256::digest(key.to_sec1_point(false).as_bytes()));
    if identity.algorithm != "ES256"
        || identity.key_id != format!("p256:{fingerprint}")
        || identity.fingerprint != fingerprint
        || identity.host_id.is_empty()
        || host_protocol::signatures::verifying_key(&identity.host_public_key).is_err()
    {
        return Err(CallError::Unreadable("invalid Hub identity".into()));
    }
    Ok(Some(identity))
}

/// The account, selected Space and local route for one first-commissioning attempt.
#[derive(Clone)]
pub(crate) struct Admission {
    scope: Option<String>,
    context: String,
    hub: Option<(String, String)>,
    route: String,
    product_host: Option<String>,
}

impl Admission {
    pub async fn capture(
        cloud: &Arc<dyn Cloud>,
        account: &crate::platform::Account,
        selected: Option<&str>,
    ) -> Result<Self, CloudError> {
        Self::capture_product(cloud, account, selected, None).await
    }

    pub async fn capture_product(
        cloud: &Arc<dyn Cloud>,
        account: &crate::platform::Account,
        selected: Option<&str>,
        product_host: Option<&str>,
    ) -> Result<Self, CloudError> {
        // Before enrollment chooses a destination, bind only the account and
        // local network. UI focus is not an authorization destination.
        let scope = selected.map(str::to_owned);
        let hub = match scope.as_deref() {
            Some(space) => tokio::time::timeout(
                REQUEST_TIMEOUT,
                select(cloud, &account.context, space, product_host),
            )
            .await
            .map_err(|_| CloudError::Unreachable("Hub selection timed out".into()))??,
            None => None,
        };
        let route = commissioning_lan(
            cloud,
            &account.context,
            if product_host.is_some() && hub.is_none() {
                None
            } else {
                scope.as_deref()
            },
            hub.as_ref(),
            None,
        )
        .await?;
        Ok(Self {
            scope,
            context: account.context.clone(),
            hub,
            route,
            product_host: product_host.map(str::to_owned),
        })
    }

    pub async fn check(
        &self,
        cloud: &Arc<dyn Cloud>,
        device: Option<std::net::SocketAddr>,
    ) -> Result<(), CloudError> {
        let account = cloud.account().await.ok_or(CloudError::NotSignedIn)?;
        if account.context != self.context {
            return Err(CloudError::AccountChanged);
        }
        let route = commissioning_lan(
            cloud,
            &self.context,
            if self.product_host.is_some() && self.hub.is_none() {
                None
            } else {
                self.scope.as_deref()
            },
            self.hub.as_ref(),
            device,
        )
        .await?;
        if route != self.route {
            return Err(CloudError::Unreachable(
                "The local network changed. Start adding the device again.".into(),
            ));
        }
        Ok(())
    }

    /// The explicitly requested first-Hub operation may publish a Hub. Adopt
    /// only that verified result, while retaining the original account/LAN.
    pub async fn accept_created_hub(
        &mut self,
        cloud: &Arc<dyn Cloud>,
        expected: &app_facade_api::pod::HubIdentity,
        device: Option<std::net::SocketAddr>,
    ) -> Result<(), CloudError> {
        if self.scope.as_deref() != Some(&expected.space_id)
            || self
                .hub
                .as_ref()
                .is_some_and(|hub| hub != &(expected.tenant_id.clone(), expected.hub_id.clone()))
        {
            return Err(CloudError::Unreachable(
                "Invalid first Hub transition".into(),
            ));
        }
        let account = cloud.account().await.ok_or(CloudError::NotSignedIn)?;
        let next = Self::capture_product(
            cloud,
            &account,
            self.scope.as_deref(),
            self.product_host.as_deref(),
        )
        .await?;
        if next.context != self.context
            || next.route != self.route
            || next.hub.as_ref() != Some(&(expected.tenant_id.clone(), expected.hub_id.clone()))
        {
            return Err(CloudError::Unreachable(
                "The first Hub or local network changed".into(),
            ));
        }
        next.check(cloud, device).await?;
        *self = next;
        Ok(())
    }

    /// Reconcile with authority before steps that can leave durable state.
    /// A ready connection to a formerly selected Hub is insufficient.
    pub async fn check_hub_selection(&self, cloud: &Arc<dyn Cloud>) -> Result<(), CloudError> {
        if let Some(scope) = self.scope.as_deref() {
            let current = tokio::time::timeout(
                REQUEST_TIMEOUT,
                select(cloud, &self.context, scope, self.product_host.as_deref()),
            )
            .await
            .map_err(|_| CloudError::Unreachable("Hub selection timed out".into()))??;
            if current != self.hub {
                return Err(CloudError::Unreachable(
                    "The selected Space's Hub changed. Start adding the device again.".into(),
                ));
            }
        }
        Ok(())
    }
}

async fn select(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    space: &str,
    product: Option<&str>,
) -> Result<Option<(String, String)>, CloudError> {
    match product {
        Some(host) => cloud.product_hub_selection(context, space, host).await,
        None => cloud.selected_hub(context, space).await,
    }
}

async fn commissioning_lan(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    scope: Option<&str>,
    hub: Option<&(String, String)>,
    device: Option<std::net::SocketAddr>,
) -> Result<String, CloudError> {
    let hub = hub.map(|(tenant, hub)| (tenant.as_str(), hub.as_str()));
    let token = tokio::time::timeout(
        REQUEST_TIMEOUT,
        cloud.commissioning_lan(context, scope, hub, device),
    )
    .await
    .map_err(|_| CloudError::Unreachable("Local network check timed out".into()))??;
    if token.is_empty() {
        return Err(CloudError::Unreachable(
            "Local network check returned no route".into(),
        ));
    }
    Ok(token)
}

/// Whether the host of `url` is `localhost`, in `127.0.0.0/8` or `::1`.
pub fn is_loopback_url(url: &str) -> bool {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next().unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    if let Ok(ip) = host.parse::<Ipv4Addr>() {
        return ip.is_loopback();
    }
    if let Ok(ip) = host.parse::<Ipv6Addr>() {
        return ip.is_loopback() || ip.to_ipv4_mapped().is_some_and(|ip| ip.is_loopback());
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn product_recovery_never_adopts_another_hosts_setup_or_removal() {
        use app_facade_api::hub::*;
        let mut view = LocalHubState {
            space_id: "space".into(),
            ..Default::default()
        };
        let select = |v: &LocalHubState| {
            selected_product_setup_hub(v, "space", "tenant".into(), "paired-host")
        };
        assert_eq!(select(&view).unwrap(), None);
        view.operation = Some(HubOperation {
            kind: HubOperationKind::Setup,
            state: HubOperationState::Failed,
            phase: HubOperationPhase::Finalizing,
            started_at: 1,
            expires_at: None,
            host_id: Some("paired-host".into()),
            hub_id: Some("hub".into()),
            removal_strategy: None,
            failure: None,
        });
        assert_eq!(select(&view).unwrap(), None);
        assert!(selected_setup_hub(&view, "space", "tenant".into()).is_err());
        view.operation.as_mut().unwrap().host_id = Some("other".into());
        assert!(select(&view).is_err());
        view.operation.as_mut().unwrap().host_id = Some("paired-host".into());
        view.operation.as_mut().unwrap().kind = HubOperationKind::Remove;
        assert!(select(&view).is_err());
        view.operation = None;
        view.configured = true;
        view.hub = Some(LocalHubView {
            cloud: HubView {
                id: "hub".into(),
                name: None,
                hub_type: "local".into(),
                host_id: "paired-host".into(),
                is_test: false,
                status: "ready".into(),
                cloud: Default::default(),
            },
            local: LocalHubRuntime {
                discovered: true,
                connected: true,
                connection_status: None,
                host_name: None,
                version: None,
            },
        });
        assert_eq!(
            select(&view).unwrap(),
            Some(("tenant".into(), "hub".into()))
        );
        view.hub.as_mut().unwrap().cloud.host_id = "other".into();
        assert!(select(&view).is_err());
    }

    #[test]
    fn recognizes_loopback_cloud_addresses() {
        for url in [
            "http://localhost:8080/api/v1",
            "ws://LOCALHOST/ws",
            "http://127.0.0.1:8080/api/v1",
            "http://127.1.2.3/api/v1",
            "http://[::1]:8080/api/v1",
            "http://[::ffff:127.0.0.1]/api/v1",
            "https://user@localhost./api/v1",
            "http://dev.localhost/api/v1",
        ] {
            assert!(is_loopback_url(url), "{url}");
        }
        for url in [
            "https://api.meowlink.example/api/v1",
            "http://192.168.0.120:8080/api/v1",
            "ws://10.0.0.2/ws",
            "http://[fe80::1]/api/v1",
            "http://localhost.example.com/",
        ] {
            assert!(!is_loopback_url(url), "{url}");
        }
    }
}
