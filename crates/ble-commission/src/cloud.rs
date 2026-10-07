//! Pod credential calls made on behalf of the signed-in user.
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Arc;
use std::time::Duration;

use app_facade_api::pod::{
    CredentialIssueRequest, CredentialIssueResponse, CredentialPodRequest,
    CredentialStatusResponse, CREDENTIAL_ISSUE_PATH, CREDENTIAL_REVOKE_PATH,
    CREDENTIAL_STATUS_PATH,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

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

async fn post<T: DeserializeOwned>(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    path: &str,
    scope_id: Option<&str>,
    body: &impl Serialize,
) -> Result<T, CallError> {
    let body =
        serde_json::to_string(body).map_err(|error| CallError::Unreadable(error.to_string()))?;
    let response = tokio::time::timeout(
        REQUEST_TIMEOUT,
        cloud.post_json(context, path, scope_id, &body),
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

pub(crate) async fn issue(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    scope_id: &str,
    request: &CredentialIssueRequest,
) -> Result<CredentialIssueResponse, CallError> {
    post(
        cloud,
        context,
        CREDENTIAL_ISSUE_PATH,
        Some(scope_id),
        request,
    )
    .await
}

pub(crate) async fn status(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    pod_id: &str,
) -> Result<CredentialStatusResponse, CallError> {
    let request = CredentialPodRequest {
        pod_id: pod_id.to_string(),
    };
    post(cloud, context, CREDENTIAL_STATUS_PATH, None, &request).await
}

pub(crate) async fn revoke(
    cloud: &Arc<dyn Cloud>,
    context: &str,
    pod_id: &str,
) -> Result<(), CallError> {
    let request = CredentialPodRequest {
        pod_id: pod_id.to_string(),
    };
    post::<Value>(cloud, context, CREDENTIAL_REVOKE_PATH, None, &request)
        .await
        .map(|_| ())
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
