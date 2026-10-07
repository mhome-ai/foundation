//! Platform-neutral BLE discovery, pod commissioning and Host provisioning for
//! MeowLink native Clients: the `/local/pod/*` and `/local/host/provision/*`
//! targets of `app-facade-api`.
//!
//! A platform supplies [`Platform`] (radio, GATT central, signed-in cloud
//! account, event sink), pushes adapter state and advertisements in, and
//! forwards facade requests to [`Commissioning::handle`].
mod actor;
mod cloud;
mod device;
mod discovery;
mod endpoints;
mod engine;
mod facade;
mod platform;
mod session;

use std::sync::Arc;

use app_facade_api::pod::AdapterStatus;
use core_api::ErrorResponse;
use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

pub use cloud::is_loopback_url;
pub use endpoints::{endpoint_uuid, fallback_endpoint_uuids};
pub use facade::{is_target, RequestError};
pub use platform::{
    Account, Advertisement, Blocking, BlockingCentral, BlockingCloud, BlockingLink, BlockingRadio,
    Central, Cloud, CloudEndpoints, CloudError, CloudResponse, EventSink, Link, LinkError,
    Platform, Radio,
};

/// The commissioning core. Dropping it stops discovery; a running session
/// finishes on its own.
pub struct Commissioning {
    engine: Arc<engine::Engine>,
    handle: Handle,
    tasks: Vec<JoinHandle<()>>,
    runtime: Option<Runtime>,
}

impl Commissioning {
    /// Runs on an existing runtime.
    pub fn new(platform: Platform, handle: Handle) -> Self {
        let (engine, tasks) = {
            let _entered = handle.enter();
            engine::Engine::start(platform)
        };
        Self {
            engine,
            handle,
            tasks,
            runtime: None,
        }
    }

    /// Runs on a runtime of its own, for callers without one.
    pub fn with_own_runtime(platform: Platform) -> std::io::Result<Self> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("ble-commission")
            .enable_all()
            .build()?;
        let mut core = Self::new(platform, runtime.handle().clone());
        core.runtime = Some(runtime);
        Ok(core)
    }

    /// Handles one facade request: `payload` is the JSON body, the result the
    /// JSON response or the facade error envelope.
    pub async fn handle(&self, target: &str, payload: &str) -> Result<String, ErrorResponse> {
        let engine = self.engine.clone();
        let (target, payload) = (target.to_string(), payload.to_string());
        self.handle
            .spawn(async move { facade::dispatch(&engine, &target, &payload).await })
            .await
            .unwrap_or_else(|error| Err(ErrorResponse::new("INTERNAL", error.to_string())))
    }

    /// [`Self::handle`] for callers outside any runtime, such as foreign
    /// threads. Must not be called from a runtime thread.
    pub fn handle_blocking(&self, target: &str, payload: &str) -> Result<String, ErrorResponse> {
        self.handle.block_on(self.handle(target, payload))
    }

    /// The platform's adapter state, whenever it changes.
    pub fn on_adapter_state(&self, status: AdapterStatus) {
        self.engine.on_adapter_state(status);
    }

    /// Every advertisement seen while scanning; others are ignored.
    pub fn on_advertisement(&self, advertisement: Advertisement) {
        self.engine.on_advertisement(&advertisement);
    }
}

impl Drop for Commissioning {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
