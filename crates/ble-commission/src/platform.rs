//! What a native Client supplies: the Bluetooth radio, GATT links, the
//! signed-in cloud account and an event sink.
//!
//! Each trait has an async form and a `Blocking*` form. Blocking methods are
//! only ever called from blocking worker threads (`spawn_blocking`), never from
//! a runtime thread or the platform's main thread, so foreign callback
//! interfaces may block on their own I/O.
use std::sync::Arc;

use app_facade_api::pod::{AdapterStatus, AuthorizationSpace, DeviceKind};
use async_trait::async_trait;

/// Why a link operation failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LinkError {
    /// The device answered with an error (an ATT error on a write or read); the
    /// link is still up.
    #[error("device rejected the request: {0}")]
    Rejected(String),
    /// The link dropped, or the platform gave up waiting.
    #[error("link lost: {0}")]
    Disconnected(String),
    /// The peripheral or an endpoint is not there.
    #[error("not found: {0}")]
    NotFound(String),
    /// The native transport itself cannot be used.
    #[error("transport unavailable: {0}")]
    Unavailable(String),
}

#[async_trait]
pub trait Radio: Send + Sync + 'static {
    /// Starts or stops scanning for the MeowLink service. A failure reports
    /// the adapter status that explains it.
    async fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus>;

    /// Opens the system page that fixes the current adapter state; `false`
    /// when the platform has none.
    async fn open_bluetooth_settings(&self) -> bool {
        false
    }
}

#[async_trait]
pub trait Central: Send + Sync + 'static {
    /// Connects and resolves the endpoints of `kind` (by `0x2901` descriptor,
    /// else [`crate::fallback_endpoint_uuids`]).
    async fn connect(
        &self,
        peripheral_id: &str,
        kind: DeviceKind,
    ) -> Result<Arc<dyn Link>, LinkError>;
}

/// Browses the Pod initialization service on the Client's own LAN. Implementations
/// report resolved/removed services through Commissioning, never through a Core.
#[async_trait]
pub trait LanDiscovery: Send + Sync + 'static {
    async fn set_browsing(&self, browsing: bool) -> Result<(), String>;
    /// The native platform must select an on-link physical network. There is
    /// deliberately no default-route fallback when that capability is missing.
    async fn network(&self, _address: std::net::SocketAddr) -> Result<LanNetwork, LinkError> {
        Err(LinkError::Unavailable(
            "Local network selection is unavailable".into(),
        ))
    }
    async fn connect(&self, address: std::net::SocketAddr) -> Result<Arc<dyn Link>, LinkError> {
        crate::lan::HttpLink::connect(address, self.network(address).await?)
    }
}

/// OS routing identity, supplied by trusted native code, never by UI input or
/// an advertisement. The platform checks that the destination is on this LAN.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanNetwork {
    pub interface_name: String,
    pub interface_index: u32,
    /// Android Network.networkHandle; binds this socket, never the process.
    pub android_network: Option<u64>,
}

pub trait BlockingLanDiscovery: Send + Sync + 'static {
    fn set_browsing(&self, browsing: bool) -> Result<(), String>;
    fn network(&self, _address: std::net::SocketAddr) -> Result<LanNetwork, LinkError> {
        Err(LinkError::Unavailable(
            "Local network selection is unavailable".into(),
        ))
    }
}

#[async_trait]
impl<T: BlockingLanDiscovery + ?Sized> LanDiscovery for Blocking<T> {
    async fn network(&self, address: std::net::SocketAddr) -> Result<LanNetwork, LinkError> {
        off_runtime(&self.0, move |lan| lan.network(address))
            .await
            .unwrap_or_else(|| Err(stopped()))
    }
    async fn set_browsing(&self, browsing: bool) -> Result<(), String> {
        off_runtime(&self.0, move |lan| lan.set_browsing(browsing))
            .await
            .unwrap_or_else(|| Err("local discovery stopped".into()))
    }
}

/// All fields are untrusted until the pairing handshake authenticates the Pod.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanAdvertisement {
    pub service_id: String,
    pub device_id: String,
    pub name: String,
    pub model: String,
    pub env: String,
    pub protocol: u32,
    pub addresses: Vec<std::net::SocketAddr>,
}

/// One connected device. Exchanges are a write with response followed by a
/// read on the endpoint's characteristic; the core never overlaps them.
#[async_trait]
pub trait Link: Send + Sync + 'static {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError>;
    async fn is_connected(&self) -> bool;
    async fn disconnect(&self);
}

/// The signed-in MeowLink account, as the authorization prompt shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub user_id: String,
    pub user_name: String,
    pub spaces: Vec<AuthorizationSpace>,
    pub active_space_id: Option<String>,
    /// Opaque to the core and handed back on every cloud call made for this
    /// account (for example a token generation), so credential issuance stays
    /// bound to the account that authorized commissioning.
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CloudError {
    #[error("not signed in")]
    NotSignedIn,
    /// The account behind `context` is no longer signed in.
    #[error("the account changed")]
    AccountChanged,
    #[error("cloud unreachable: {0}")]
    Unreachable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudResponse {
    pub status: u16,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudEndpoints {
    /// `https://…/api/v1`
    pub api: String,
    pub ws: String,
}

#[async_trait]
pub trait Cloud: Send + Sync + 'static {
    /// Native LAN claim of the Host whose key was read through the paired BLE
    /// session. Implementations must pin this key before sending a claim/token.
    /// Returns only after ordinary cloud registration has committed.
    async fn claim_paired_host(
        &self,
        _context: &str,
        _host_id: &str,
        _host_key: &str,
        _claim_token: &str,
        _space_id: Option<&str>,
    ) -> Result<(), CloudError> {
        Err(CloudError::Unreachable(
            "Paired Host authorization is unavailable".into(),
        ))
    }
    async fn setup_product_hub(
        &self,
        _context: &str,
        _scope: &str,
        _host_id: &str,
        _host_key: &str,
    ) -> Result<(), CloudError> {
        Err(CloudError::Unreachable(
            "Product Hub setup is unavailable".into(),
        ))
    }
    async fn selected_hub(
        &self,
        _context: &str,
        _space_id: &str,
    ) -> Result<Option<(String, String)>, CloudError> {
        Err(CloudError::Unreachable(
            "Hub selection is unavailable".into(),
        ))
    }
    /// First-Hub recovery may resume only this paired product's setup.
    async fn product_hub_selection(
        &self,
        _context: &str,
        _space_id: &str,
        _host_id: &str,
    ) -> Result<Option<(String, String)>, CloudError> {
        Err(CloudError::Unreachable(
            "Product Hub recovery is unavailable".into(),
        ))
    }
    /// A stable token for the current native LAN and local Hub transport.
    /// A configured Hub must be authenticated and reached directly over this
    /// LAN. When there is no Hub, only the native network contributes.
    async fn commissioning_lan(
        &self,
        _context: &str,
        _space_id: Option<&str>,
        _hub: Option<(&str, &str)>,
        _device: Option<std::net::SocketAddr>,
    ) -> Result<String, CloudError> {
        Err(CloudError::Unreachable(
            "Local commissioning network is unavailable".into(),
        ))
    }
    async fn account(&self) -> Option<Account>;
    /// `POST {api}/{path}` with the account's bearer token, the `SpaceId`
    /// header when `space_id` is set, and a JSON body.
    async fn post_json(
        &self,
        account_context: &str,
        path: &str,
        space_id: Option<&str>,
        body: &str,
    ) -> Result<CloudResponse, CloudError>;
    async fn client_id(&self) -> Option<String>;
    async fn cloud_endpoints(&self) -> CloudEndpoints;
}

/// Receives `/local/*` events with their JSON payload.
pub trait EventSink: Send + Sync + 'static {
    fn publish(&self, target: &str, payload: &str);
}

/// A nearby advertisement as the platform saw it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Advertisement {
    pub peripheral_id: String,
    pub local_name: Option<String>,
    /// Manufacturer data for company `0xFFFF`, without the company ID.
    pub manufacturer_data: Option<Vec<u8>>,
    pub service_uuids: Vec<String>,
    pub rssi: Option<i16>,
}

pub trait BlockingRadio: Send + Sync + 'static {
    fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus>;
    fn open_bluetooth_settings(&self) -> bool {
        false
    }
}

pub trait BlockingCentral: Send + Sync + 'static {
    fn connect(
        &self,
        peripheral_id: String,
        kind: DeviceKind,
    ) -> Result<Arc<dyn BlockingLink>, LinkError>;
}

pub trait BlockingLink: Send + Sync + 'static {
    fn exchange(&self, endpoint: String, request: Vec<u8>) -> Result<Vec<u8>, LinkError>;
    fn is_connected(&self) -> bool;
    fn disconnect(&self);
}

pub trait BlockingCloud: Send + Sync + 'static {
    fn claim_paired_host(
        &self,
        _context: String,
        _host_id: String,
        _host_key: String,
        _claim_token: String,
        _space_id: Option<String>,
    ) -> Result<(), CloudError> {
        Err(CloudError::Unreachable(
            "Paired Host authorization is unavailable".into(),
        ))
    }
    fn setup_product_hub(
        &self,
        _context: String,
        _scope: String,
        _host_id: String,
        _host_key: String,
    ) -> Result<(), CloudError> {
        Err(CloudError::Unreachable(
            "Product Hub setup is unavailable".into(),
        ))
    }
    fn selected_hub(
        &self,
        _context: String,
        _space_id: String,
    ) -> Result<Option<(String, String)>, CloudError> {
        Err(CloudError::Unreachable(
            "Hub selection is unavailable".into(),
        ))
    }
    fn product_hub_selection(
        &self,
        _context: String,
        _space_id: String,
        _host_id: String,
    ) -> Result<Option<(String, String)>, CloudError> {
        Err(CloudError::Unreachable(
            "Product Hub recovery is unavailable".into(),
        ))
    }
    fn commissioning_lan(
        &self,
        _context: String,
        _space_id: Option<String>,
        _hub: Option<(String, String)>,
        _device: Option<std::net::SocketAddr>,
    ) -> Result<String, CloudError> {
        Err(CloudError::Unreachable(
            "Local commissioning network is unavailable".into(),
        ))
    }
    fn account(&self) -> Option<Account>;
    fn post_json(
        &self,
        account_context: String,
        path: String,
        space_id: Option<String>,
        body: String,
    ) -> Result<CloudResponse, CloudError>;
    fn client_id(&self) -> Option<String>;
    fn cloud_endpoints(&self) -> CloudEndpoints;
}

/// Runs a blocking implementation on the blocking pool.
pub struct Blocking<T: ?Sized>(pub Arc<T>);

impl<T: ?Sized> Clone for Blocking<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

async fn off_runtime<T: ?Sized + Send + Sync + 'static, R: Send + 'static>(
    target: &Arc<T>,
    call: impl FnOnce(&T) -> R + Send + 'static,
) -> Option<R> {
    let target = target.clone();
    tokio::task::spawn_blocking(move || call(&target))
        .await
        .ok()
}

#[async_trait]
impl<T: BlockingRadio + ?Sized> Radio for Blocking<T> {
    async fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus> {
        off_runtime(&self.0, move |radio| radio.set_scanning(scanning))
            .await
            .unwrap_or_else(|| Err(stopped_adapter()))
    }

    async fn open_bluetooth_settings(&self) -> bool {
        off_runtime(&self.0, |radio| radio.open_bluetooth_settings())
            .await
            .unwrap_or(false)
    }
}

fn stopped_adapter() -> AdapterStatus {
    AdapterStatus {
        state: app_facade_api::pod::AdapterState::Unavailable,
        message: None,
    }
}

fn stopped() -> LinkError {
    LinkError::Disconnected("the platform call did not return".into())
}

#[async_trait]
impl<T: BlockingCentral + ?Sized> Central for Blocking<T> {
    async fn connect(
        &self,
        peripheral_id: &str,
        kind: DeviceKind,
    ) -> Result<Arc<dyn Link>, LinkError> {
        let peripheral_id = peripheral_id.to_string();
        let link = off_runtime(&self.0, move |central| central.connect(peripheral_id, kind))
            .await
            .unwrap_or_else(|| Err(stopped()))?;
        Ok(Arc::new(Blocking(link)))
    }
}

#[async_trait]
impl<T: BlockingLink + ?Sized> Link for Blocking<T> {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError> {
        let endpoint = endpoint.to_string();
        let request = request.to_vec();
        off_runtime(&self.0, move |link| link.exchange(endpoint, request))
            .await
            .unwrap_or_else(|| Err(stopped()))
    }

    async fn is_connected(&self) -> bool {
        off_runtime(&self.0, |link| link.is_connected())
            .await
            .unwrap_or(false)
    }

    async fn disconnect(&self) {
        off_runtime(&self.0, |link| link.disconnect()).await;
    }
}

#[async_trait]
impl<T: BlockingCloud + ?Sized> Cloud for Blocking<T> {
    async fn claim_paired_host(
        &self,
        context: &str,
        host_id: &str,
        host_key: &str,
        claim_token: &str,
        space_id: Option<&str>,
    ) -> Result<(), CloudError> {
        let (context, host_id, host_key, claim_token, space_id) = (
            context.to_owned(),
            host_id.to_owned(),
            host_key.to_owned(),
            claim_token.to_owned(),
            space_id.map(str::to_owned),
        );
        off_runtime(&self.0, move |cloud| {
            cloud.claim_paired_host(context, host_id, host_key, claim_token, space_id)
        })
        .await
        .unwrap_or_else(|| Err(CloudError::Unreachable("Host authorization stopped".into())))
    }
    async fn setup_product_hub(
        &self,
        context: &str,
        scope: &str,
        host_id: &str,
        host_key: &str,
    ) -> Result<(), CloudError> {
        let (context, scope, host_id, host_key) = (
            context.to_owned(),
            scope.to_owned(),
            host_id.to_owned(),
            host_key.to_owned(),
        );
        off_runtime(&self.0, move |cloud| {
            cloud.setup_product_hub(context, scope, host_id, host_key)
        })
        .await
        .unwrap_or_else(|| Err(CloudError::Unreachable("Product setup stopped".into())))
    }
    async fn selected_hub(
        &self,
        context: &str,
        space_id: &str,
    ) -> Result<Option<(String, String)>, CloudError> {
        let (context, space_id) = (context.to_owned(), space_id.to_owned());
        off_runtime(&self.0, move |cloud| cloud.selected_hub(context, space_id))
            .await
            .unwrap_or_else(|| Err(CloudError::Unreachable("Hub selection stopped".into())))
    }

    async fn product_hub_selection(
        &self,
        context: &str,
        space_id: &str,
        host_id: &str,
    ) -> Result<Option<(String, String)>, CloudError> {
        let (context, space_id, host_id) =
            (context.to_owned(), space_id.to_owned(), host_id.to_owned());
        off_runtime(&self.0, move |cloud| {
            cloud.product_hub_selection(context, space_id, host_id)
        })
        .await
        .unwrap_or_else(|| {
            Err(CloudError::Unreachable(
                "Product Hub selection stopped".into(),
            ))
        })
    }

    async fn commissioning_lan(
        &self,
        context: &str,
        space_id: Option<&str>,
        hub: Option<(&str, &str)>,
        device: Option<std::net::SocketAddr>,
    ) -> Result<String, CloudError> {
        let context = context.to_owned();
        let space_id = space_id.map(str::to_owned);
        let hub = hub.map(|(tenant, hub)| (tenant.to_owned(), hub.to_owned()));
        off_runtime(&self.0, move |cloud| {
            cloud.commissioning_lan(context, space_id, hub, device)
        })
        .await
        .unwrap_or_else(|| {
            Err(CloudError::Unreachable(
                "Local network check stopped".into(),
            ))
        })
    }

    async fn account(&self) -> Option<Account> {
        off_runtime(&self.0, |cloud| cloud.account())
            .await
            .flatten()
    }

    async fn post_json(
        &self,
        account_context: &str,
        path: &str,
        space_id: Option<&str>,
        body: &str,
    ) -> Result<CloudResponse, CloudError> {
        let (context, path, space_id, body) = (
            account_context.to_string(),
            path.to_string(),
            space_id.map(str::to_string),
            body.to_string(),
        );
        off_runtime(&self.0, move |cloud| {
            cloud.post_json(context, path, space_id, body)
        })
        .await
        .unwrap_or_else(|| {
            Err(CloudError::Unreachable(
                "the platform call did not return".into(),
            ))
        })
    }

    async fn client_id(&self) -> Option<String> {
        off_runtime(&self.0, |cloud| cloud.client_id())
            .await
            .flatten()
    }

    async fn cloud_endpoints(&self) -> CloudEndpoints {
        off_runtime(&self.0, |cloud| cloud.cloud_endpoints())
            .await
            .unwrap_or(CloudEndpoints {
                api: String::new(),
                ws: String::new(),
            })
    }
}

/// The platform implementations a [`crate::Commissioning`] runs on.
#[derive(Clone)]
pub struct Platform {
    pub radio: Arc<dyn Radio>,
    pub lan: Arc<dyn LanDiscovery>,
    pub central: Arc<dyn Central>,
    pub cloud: Arc<dyn Cloud>,
    pub events: Arc<dyn EventSink>,
}

impl Platform {
    /// For foreign implementations whose methods block.
    pub fn blocking(
        radio: Arc<dyn BlockingRadio>,
        lan: Arc<dyn BlockingLanDiscovery>,
        central: Arc<dyn BlockingCentral>,
        cloud: Arc<dyn BlockingCloud>,
        events: Arc<dyn EventSink>,
    ) -> Self {
        Self {
            radio: Arc::new(Blocking(radio)),
            lan: Arc::new(Blocking(lan)),
            central: Arc::new(Blocking(central)),
            cloud: Arc::new(Blocking(cloud)),
            events,
        }
    }
}
