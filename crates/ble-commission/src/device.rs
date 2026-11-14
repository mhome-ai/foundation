//! Talking to one device over a platform [`Link`]: protocomm transport with
//! timeouts, the version check, and response parsing.
use std::sync::Arc;
use std::time::Duration;

use app_facade_api::host_provision::HostDeviceInfo;
use app_facade_api::pod::{
    DeviceKind, PodDeviceInfo, PodInfo, WifiNetwork, APP_INFO_LABEL, APP_INFO_LOCKED,
    APP_INFO_LOCKED_FOR_MS, SECURITY_VERSION,
};
use async_trait::async_trait;
use protocomm::client::WIFI_SCAN_ENDPOINT;
use protocomm::proto::WifiAuthMode;
use protocomm::sec2::MAX_PATCH_VERSION;
use protocomm::{ProtocommClient, ScanEntry, Transport, TransportError, VersionInfo};
use serde_json::Value;

use crate::platform::{Central, Link, LinkError};

pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(20);
pub(crate) const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(20);
/// A Host may spend most of the default exchange timeout scanning.
pub(crate) const SCAN_EXCHANGE_TIMEOUT: Duration = Duration::from_secs(30);

pub(crate) struct LinkTransport {
    pub link: Arc<dyn Link>,
}

#[async_trait]
impl Transport for LinkTransport {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, TransportError> {
        let limit = if endpoint == WIFI_SCAN_ENDPOINT {
            SCAN_EXCHANGE_TIMEOUT
        } else {
            EXCHANGE_TIMEOUT
        };
        match tokio::time::timeout(limit, self.link.exchange(endpoint, request)).await {
            Err(_) => Err(TransportError::Disconnected(format!(
                "{endpoint} timed out"
            ))),
            Ok(Ok(response)) => Ok(response),
            Ok(Err(LinkError::Rejected(detail))) => Err(TransportError::Rejected(detail)),
            Ok(Err(error)) => Err(TransportError::Disconnected(error.to_string())),
        }
    }
}

pub(crate) type Client = ProtocommClient<LinkTransport>;

pub(crate) struct Opened {
    pub client: Arc<Client>,
    pub link: Arc<dyn Link>,
    pub patch_version: u32,
    pub locked: bool,
}

pub(crate) enum OpenError {
    Unavailable(String),
    Link(String),
    /// The device does not speak this protocol or kind.
    Incompatible(String),
}

pub(crate) fn capability(kind: DeviceKind) -> &'static str {
    match kind {
        DeviceKind::Pod => app_facade_api::pod::APP_CAPABILITY,
        DeviceKind::Host => app_facade_api::host_provision::APP_CAPABILITY,
    }
}

pub(crate) async fn open(
    central: &Arc<dyn Central>,
    peripheral_id: &str,
    kind: DeviceKind,
) -> Result<Opened, OpenError> {
    let connecting = central.clone();
    let id = peripheral_id.to_owned();
    let mut task = tokio::spawn(async move { connecting.connect(&id, kind).await });
    let link = match tokio::time::timeout(CONNECT_TIMEOUT, &mut task).await {
        Err(_) => {
            // Platform connects have their own bounded timeout. Do not abandon
            // one: a late connection must be closed before releasing the radio.
            if let Ok(Ok(link)) = task.await {
                link.disconnect().await;
            }
            return Err(OpenError::Link("connection timed out".into()));
        }
        Ok(Err(error)) => return Err(OpenError::Link(error.to_string())),
        Ok(Ok(Err(LinkError::Unavailable(detail)))) => return Err(OpenError::Unavailable(detail)),
        Ok(Ok(Err(error))) => return Err(OpenError::Link(error.to_string())),
        Ok(Ok(Ok(link))) => link,
    };
    open_link(link, kind).await
}

pub(crate) async fn open_link(link: Arc<dyn Link>, kind: DeviceKind) -> Result<Opened, OpenError> {
    let client = Arc::new(ProtocommClient::new(LinkTransport { link: link.clone() }));
    let version = match client.version().await {
        Ok(version) => version,
        Err(error) => {
            link.disconnect().await;
            return Err(OpenError::Link(error.to_string()));
        }
    };
    let patch_version = version.security_patch_version();
    if version.security_version() != Some(SECURITY_VERSION)
        || patch_version != MAX_PATCH_VERSION
        || !version.has_app_capability(APP_INFO_LABEL, capability(kind))
    {
        link.disconnect().await;
        return Err(OpenError::Incompatible(version.raw.to_string()));
    }
    Ok(Opened {
        client,
        link,
        patch_version,
        locked: locked(&version),
    })
}

fn locked(version: &VersionInfo) -> bool {
    version.app_flag(APP_INFO_LABEL, APP_INFO_LOCKED)
        || version
            .raw
            .get(APP_INFO_LABEL)
            .and_then(|app| app.get(APP_INFO_LOCKED_FOR_MS))
            .is_some_and(Value::is_u64)
}

/// `None` when the version could not be read.
pub(crate) async fn is_locked(client: &Client) -> Option<bool> {
    client.version().await.ok().map(|version| locked(&version))
}

pub(crate) fn pod_info(info: &Value) -> Option<(PodInfo, PodDeviceInfo)> {
    let parsed: PodInfo = serde_json::from_value(info.clone()).ok()?;
    if parsed.device_id.is_empty() || parsed.model.is_empty() || parsed.firmware_version.is_empty()
    {
        return None;
    }
    let device = PodDeviceInfo {
        dependent_host: parsed
            .device_binding
            .as_ref()
            .is_some_and(|binding| binding.host.is_some()),
        device_id: parsed.device_id.clone(),
        model: parsed.model.clone(),
        firmware_version: parsed.firmware_version.clone(),
        wifi_configured: parsed.wifi_configured,
    };
    Some((parsed, device))
}

/// Validate the compact BLE identity and expand its canonical cryptographic
/// fields for the later same-device LAN comparison. No binding is inferred.
pub(crate) fn pod_network_info(info: &Value) -> Option<(PodInfo, PodDeviceInfo, bool)> {
    use app_facade_api::pod::{PodIdentity, PodNetworkInfo, CUSTOM_ENDPOINT_MAX_BYTES};
    use sha2::{Digest, Sha256};
    if serde_json::to_vec(info).ok()?.len() > CUSTOM_ENDPOINT_MAX_BYTES {
        return None;
    }
    let network: PodNetworkInfo = serde_json::from_value(info.clone()).ok()?;
    if network.protocol != 1
        || network.env != core_api::RuntimeEnv::current().as_str()
        || network.device_id.is_empty()
        || network.model.is_empty()
        || network.firmware_version.is_empty()
        || network
            .mode
            .as_deref()
            .is_some_and(|mode| mode != app_facade_api::pod::POD_MODE_REPROVISION)
    {
        return None;
    }
    let [x, y] = network.public_key;
    let key = host_protocol::signatures::verifying_key(&core_api::host::auth::PublicKey {
        x: x.clone(),
        y: y.clone(),
    })
    .ok()?;
    let fingerprint = hex::encode(Sha256::digest(key.to_sec1_point(false).as_bytes()));
    let device = PodDeviceInfo {
        device_id: network.device_id.clone(),
        model: network.model.clone(),
        firmware_version: network.firmware_version.clone(),
        wifi_configured: network.wifi_configured,
        dependent_host: network.dependent_host,
    };
    Some((
        PodInfo {
            protocol: network.protocol,
            env: network.env,
            device_id: network.device_id,
            model: network.model,
            platform: String::new(),
            firmware_version: network.firmware_version,
            identity: PodIdentity {
                key_id: format!("p256:{fingerprint}"),
                alg: "ES256".into(),
                x,
                y,
                fingerprint,
            },
            wifi_configured: network.wifi_configured,
            state: network.state,
            mode: network.mode,
            device_binding: None,
            pod_id: None,
            client_mode: None,
        },
        device,
        network.dependent_host,
    ))
}

pub(crate) fn host_info(info: &Value) -> Option<HostDeviceInfo> {
    let text = |key: &str| {
        info.get(key)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    Some(HostDeviceInfo {
        host_id: text("hostId")?,
        name: text("name")?,
        product_id: text("productId"),
        firmware_version: text("firmwareVersion")?,
        fingerprint: text("fingerprint")?,
        wifi_configured: info.get("wifiConfigured").and_then(Value::as_bool)?,
        authorization_mode: serde_json::from_value(info.get("authorizationMode")?.clone()).ok()?,
        onboarding_required: info.get("onboardingRequired").and_then(Value::as_bool)?,
    })
}

pub(crate) fn host_identity(
    info: &Value,
    host_id: &str,
) -> Option<core_api::host::auth::PublicKey> {
    if info.get("hostId")?.as_str()? != host_id {
        return None;
    }
    let [x, y]: [String; 2] = serde_json::from_value(info.get("publicKey")?.clone()).ok()?;
    let key = core_api::host::auth::PublicKey { x, y };
    host_protocol::signatures::verifying_key(&key).ok()?;
    Some(key)
}

/// One entry per SSID, strongest first; hidden and enterprise networks dropped.
pub(crate) fn networks(entries: Vec<ScanEntry>) -> Vec<WifiNetwork> {
    let mut networks: Vec<WifiNetwork> = Vec::new();
    for entry in entries {
        if entry.ssid.is_empty() || entry.auth == WifiAuthMode::Wpa2Enterprise {
            continue;
        }
        let rssi = entry.rssi.clamp(i16::MIN.into(), i16::MAX.into()) as i16;
        let secured = entry.auth != WifiAuthMode::Open;
        match networks
            .iter_mut()
            .find(|network| network.ssid == entry.ssid)
        {
            Some(existing) if existing.rssi >= rssi => {}
            Some(existing) => {
                existing.rssi = rssi;
                existing.secured = secured;
            }
            None => networks.push(WifiNetwork {
                ssid: entry.ssid,
                rssi,
                secured,
            }),
        }
    }
    networks.sort_by_key(|network| std::cmp::Reverse(network.rssi));
    networks
}

pub(crate) fn valid_code(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|byte| byte.is_ascii_digit())
}

/// WPA passphrases are 8 to 63 printable characters, or 64 hex digits.
pub(crate) fn valid_wifi(ssid: &str, password: &str) -> bool {
    let ssid_ok = (1..=32).contains(&ssid.len());
    let password_ok = password.is_empty()
        || ((8..=63).contains(&password.len()) && password.is_ascii())
        || (password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()));
    ssid_ok && password_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_results_keep_the_strongest_entry_per_network() {
        let entry = |ssid: &str, rssi: i32, auth: WifiAuthMode| ScanEntry {
            ssid: ssid.into(),
            rssi,
            channel: 1,
            auth,
        };
        let found = networks(vec![
            entry("home", -70, WifiAuthMode::Wpa2Psk),
            entry("home", -50, WifiAuthMode::Wpa2Psk),
            entry("", -40, WifiAuthMode::Open),
            entry("office", -45, WifiAuthMode::Wpa2Enterprise),
            entry("cafe", -60, WifiAuthMode::Open),
        ]);
        assert_eq!(
            found,
            vec![
                WifiNetwork {
                    ssid: "home".into(),
                    rssi: -50,
                    secured: true
                },
                WifiNetwork {
                    ssid: "cafe".into(),
                    rssi: -60,
                    secured: false
                },
            ]
        );
    }

    #[test]
    fn wifi_credentials_follow_the_wpa_limits() {
        assert!(valid_wifi("home", ""));
        assert!(valid_wifi("home", "12345678"));
        assert!(valid_wifi("home", &"a".repeat(63)));
        assert!(valid_wifi("home", &"ab".repeat(32)));
        assert!(!valid_wifi("home", &"g".repeat(64)));
        assert!(!valid_wifi("home", "1234567"));
        assert!(!valid_wifi("", "12345678"));
        assert!(!valid_wifi(&"s".repeat(33), "12345678"));
        assert!(valid_code("042137"));
        assert!(!valid_code("42137"));
        assert!(!valid_code("04213a"));
    }

    #[test]
    fn host_info_requires_the_contract_fields() {
        let info = serde_json::json!({
            "protocol": 1,
            "hostId": "host-1",
            "name": "Kitchen Host",
            "firmwareVersion": "1.0.7",
            "fingerprint": "ab",
            "wifiConfigured": false,
            "authorizationMode": "admin",
            "onboardingRequired": true,
        });
        let parsed = host_info(&info).expect("complete");
        assert_eq!(parsed.product_id, None);
        let mut missing = info.clone();
        missing.as_object_mut().unwrap().remove("fingerprint");
        assert!(host_info(&missing).is_none());
        for field in ["authorizationMode", "onboardingRequired"] {
            let mut missing = info.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(host_info(&missing).is_none());
        }
    }
    #[test]
    fn ble_network_info_fits_att_and_derives_the_lan_identity_without_binding() {
        use app_facade_api::pod::{PodNetworkError, PodNetworkInfo, CUSTOM_ENDPOINT_MAX_BYTES};
        let key = host_protocol::signatures::new_key();
        let public = host_protocol::signatures::public_key(key.verifying_key());
        let network = PodNetworkInfo {
            protocol: 1,
            env: core_api::RuntimeEnv::current().as_str().into(),
            device_id: "d".repeat(63),
            model: "LinuxPod".into(),
            firmware_version: "v".repeat(31),
            public_key: [public.x.clone(), public.y.clone()],
            wifi_configured: false,
            state: "wifi_failed".into(),
            dependent_host: true,
            mode: Some("reprovision".into()),
            error: Some(PodNetworkError {
                code: "wifi_hub_unverified".into(),
            }),
        };
        let bytes = serde_json::to_vec(&network).unwrap();
        assert!(bytes.len() <= CUSTOM_ENDPOINT_MAX_BYTES);
        let value = serde_json::to_value(network).unwrap();
        let (info, device, dependent) = pod_network_info(&value).unwrap();
        assert!(dependent && device.dependent_host && info.reprovision());
        assert!(
            info.device_binding.is_none() && info.pod_id.is_none() && info.client_mode.is_none()
        );
        assert_eq!(info.identity.x, public.x);
        use sha2::{Digest, Sha256};
        let digest = hex::encode(Sha256::digest(
            key.verifying_key().to_sec1_point(false).as_bytes(),
        ));
        assert_eq!(info.identity.key_id, format!("p256:{digest}"));
        assert_eq!(info.identity.fingerprint, digest);
        let mut wrong = value.clone();
        wrong["publicKey"][0] = serde_json::json!("not-a-key");
        assert!(pod_network_info(&wrong).is_none());
        let mut wrong = value.clone();
        wrong["protocol"] = serde_json::json!(2);
        assert!(pod_network_info(&wrong).is_none());
        let mut wrong = value;
        wrong["deviceBinding"] = serde_json::json!({"host":"injected"});
        assert!(pod_network_info(&wrong).is_none());
    }
}
