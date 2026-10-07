# mhome-ble-commission

Platform-neutral BLE discovery, pod commissioning and Host provisioning for
MeowLink native Clients. It implements the `/local/pod/*` and
`/local/host/provision/*` targets of `mhome-app-facade-api` once, so desktop and
mobile Clients only supply a radio, a GATT central, the signed-in cloud account
and an event sink.

## Entry points

```rust
let core = Commissioning::new(platform, tokio::runtime::Handle::current());
// or, without a runtime of your own:
let core = Commissioning::with_own_runtime(platform)?;

core.on_adapter_state(status);          // whenever the adapter changes
core.on_advertisement(advertisement);   // every advertisement while scanning
let json = core.handle(target, payload).await?;     // facade request
let json = core.handle_blocking(target, payload)?;  // from a foreign thread
```

`is_target(target)` tells whether a facade target belongs here. Errors come back
as the `core-api` `ErrorResponse` envelope: `BAD_REQUEST` for unreadable bodies,
`UNSUPPORTED` for unknown targets, `PRECONDITION_FAIL` with a stable
`details.reason` (`RequestErrorReason`) when the request is refused.

Scanning only starts once the platform has reported a `ready` adapter, a
discovery lease is live and no session holds the radio. Dropping
`Commissioning` stops its tasks.

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
- `LinkError::Rejected` means the device answered the write with an error (an
  ATT error from ESP-IDF); any other variant means the link is unusable.
- `Cloud::post_json` posts to a path under the cloud API base with the account
  identified by `account_context`, scoped to the Space `scope_id` when given. It
  returns `CloudError::AccountChanged` when that account is no longer signed in,
  so a revoke after sign-out is never sent with another account.
- `cloud_endpoints` are the API and WebSocket bases the pod will use; loopback
  addresses are refused with `cloud_unreachable_for_device`.
- `EventSink::publish` is called in order from the blocking pool with the
  `/local/pod/discovery/changed`, `/local/pod/commission/changed` and
  `/local/host/provision/changed` payloads. Discovery events are coalesced to
  at most one per second.

## Sessions

At most one pod session and one Host session are kept, and only one of them
may hold the radio: starting either while the other is still running (or still
disconnecting) is refused with `session_active` or `busy`. A session ends
`cancelled` when its owner lease is not renewed and `timed_out` after its
lifetime, except while a credential is being delivered or activated, which run
to a conclusion. A pod credential that was issued but did not end up active is
revoked with the account that issued it.
