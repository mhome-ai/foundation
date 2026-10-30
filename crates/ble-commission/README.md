# mhome-ble-commission

Platform-neutral BLE and LAN discovery, pod commissioning and Host provisioning for
MeowLink native Clients. It implements the `/local/pod/*` and
`/local/host/provision/*` targets of `mhome-app-facade-api` once, so desktop and
mobile Clients only supply a radio, a GATT central, a native LAN browser, the signed-in cloud account
and an event sink.

## Entry points

```rust
let core = Commissioning::new(platform, tokio::runtime::Handle::current());
// or, without a runtime of your own:
let core = Commissioning::with_own_runtime(platform)?;

core.on_adapter_state(status);          // whenever the adapter changes
core.on_advertisement(advertisement);   // every advertisement while scanning
core.on_lan_advertisement(advertisement); // resolved _meow-pod-init service
core.on_lan_removed(service_id);
core.on_lan_error(message);             // asynchronous browser/permission failure
let json = core.handle(target, payload).await?;     // facade request
let json = core.handle_blocking(target, payload)?;  // from a foreign thread
```

`is_target(target)` tells whether a facade target belongs here. Errors come back
as the `core-api` `ErrorResponse` envelope: `BAD_REQUEST` for unreadable bodies,
`UNSUPPORTED` for unknown targets, `PRECONDITION_FAIL` with a stable
`details.reason` (`RequestErrorReason`) when the request is refused.

Scanning only starts once the platform has reported a `ready` adapter, a
discovery lease is live and no session holds the radio. Dropping
`Commissioning` stops its tasks. LAN browsing follows discovery leases and active
Pod sessions, independently of Bluetooth availability. `LanDiscovery::set_browsing`
controls the native DNS-SD browser; HTTP/SRP is implemented once in this crate.

## Platform traits

All are `Send + Sync`. Async implementations use `#[async_trait]`;
implementations whose calls block (UniFFI foreign traits, for example) implement
the `Blocking*` variant and pass through `Platform::blocking`, which runs every
call on the blocking pool.

```rust
trait Radio {
    async fn set_scanning(&self, scanning: bool) -> Result<(), AdapterStatus>;
    async fn open_bluetooth_settings(&self) -> bool { false }
}

trait Central {
    async fn connect(&self, peripheral_id: &str, kind: DeviceKind)
        -> Result<Arc<dyn Link>, LinkError>;
}

trait Link {
    async fn exchange(&self, endpoint: &str, request: &[u8]) -> Result<Vec<u8>, LinkError>;
    async fn is_connected(&self) -> bool;
    async fn disconnect(&self);
}

trait Cloud {
    async fn account(&self) -> Option<Account>;
    async fn post_json(&self, account_context: &str, path: &str,
        scope_id: Option<&str>, body: &str) -> Result<CloudResponse, CloudError>;
    async fn client_id(&self) -> Option<String>;
    async fn cloud_endpoints(&self) -> CloudEndpoints;
}

trait EventSink {
    fn publish(&self, target: &str, payload: &str);
}
```

- `Central::connect` maps endpoint names to characteristics by their `0x2901`
  user descriptions; `fallback_endpoint_uuids(kind)` lists the UUIDs to use
  when a device has none. `Link::exchange` writes then reads one
  endpoint.
- `Link::exchange` must use long (offset) reads: a Host answer can fill the
  512-byte attribute.
- `LinkError::Rejected` means the device answered the write with an error (an
  ATT error, including the Insufficient Authorization a locked Host returns);
  any other variant means the link is unusable.
- `Cloud::post_json` posts to a path under the cloud API base with the account
  identified by `account_context`, scoped to the Space `scope_id` when given. It
  returns `CloudError::AccountChanged` when that account is no longer signed in,
  so credential issuance is never sent with another account.
- `cloud_endpoints` are the API and WebSocket bases the pod will use; loopback
  addresses are refused with `cloud_unreachable_for_device`.
- `EventSink::publish` is called in order from the blocking pool with the
  `/local/pod/discovery/changed`, `/local/pod/commission/changed` and
  `/local/host/provision/changed` payloads. Discovery events are coalesced to
  at most one per second.

## Sessions

At most one pod session and one Host session are kept, and only one of them
may hold the radio: starting either while the other is still running (or still
disconnecting) is refused with `session_active` or `busy`. The native core owns a session until success, failure or its finite deadline;
leaving a page does not cancel it and callers do not renew it. There is no
user cancellation endpoint. Only the device's `commissioned` response confirms
success. Lost delivery/activation confirmation fails and releases the link.
There is no cloud revocation call; unused pending credentials expire. Select the LAN candidate again to retry activation; the device retains a saved
network. The secure channel verifies the full device ID, environment and (after
BLE handoff) identity key. A fresh handshake uses new cipher state, while the
entered code is held temporarily to avoid prompting twice. Completed or in-flight
activation is read before any new credential issuance. Completed enrollment
snapshots contain `podId` and `clientMode` (`user` or `space`), including after
reconnecting to a completed attempt. Reconnection checks both against the
credential being confirmed; a missing or changed mode fails rather than guessing.


A pod whose `pod-info` reports `"mode":"reprovision"` is in a Wi-Fi change
window: the session only joins the new network, waits for the pod to save it,
then completes with `"mode":"reprovision"` and no `podId`. No credential is
issued and the cloud is not called.

## Hub initialization

`hub::Commissioning` implements the native `/local/hub/*` setup facade for desktop,
Android and iOS. `hub::Platform` supplies account-scoped discovery, cloud state,
cloud authorization, direct LAN HTTPS and business-connection scheduling. This
protocol is independent of Host administration: a Space owner can initialize a
Hub on an already running Core without becoming that machine's administrator.

The engine verifies the Core's signed challenge against the actual TLS peer and
runtime Space policy before sending secrets. Platform adapters reject route/login
replacement, redirects and proxy use. Confirm starts one fixed ten-minute attempt;
fresh retry retires the previous local job. Cloud/Core attempt IDs fence late work.
Status reconciles the exact committed Hub even if the setup response was lost.

`/local/hub/setup/start` returns a tagged result: `PAIRING_REQUIRED` carries the
pairing session/code, while `RUNNING` and `COMPLETED` carry the exact `hubId`.
A retry reads cloud state first. Before the commit checkpoint it starts a fresh
paired attempt; at `FINALIZING` it uses the same cloud initiate entrypoint to
finish the recorded Hub without LAN discovery or new credentials. Completed
retries require the same Host and Hub. The cloud always rechecks Space ownership.
The command has the shared Core-request deadline (60 seconds); adapters receive
the cloud call's budget explicitly. A running migration is observed separately
within the existing ten-minute setup deadline. Failure preserves the recorded
phase for an explicit retry, rather than starting automatic recovery.
