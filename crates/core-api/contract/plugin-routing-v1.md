# Plugin routing

Authenticated Plugins use the current Space's MeowCore connection to communicate
with another bound Plugin. The shared types are `node::plugin`. This is an
internal service protocol, not an `/app/*` Facade or a device capability.

| Core ingress | Message | Result |
| --- | --- | --- |
| `/plugin/instance/list` | Request `{nodeType?}` | `{instances: [{address, access}]}` |
| `/plugin/request` | Request `{to, target, version, payload, timeoutMs?}` | Destination's unchanged RuntimeResponse, or transport error |
| `/plugin/event` | One-way event `{to, target, version, payload}` | No response |

`to` is `{nodeType, nodeId}`. Both fields are required for delivery; discovery
never implicitly chooses an instance. Listing returns only current-Space online
instances with an admitted, non-empty Plugin interface description. An unavailable
description is not a usable interface. The caller retains its exact selected
instance across calls; there is no automatic instance failover.

Core derives tenant, Space and sender from the authenticated connection. Sender
inputs cannot contain `from`, `spaceId`, `context` or transport `control`. Routing
never crosses a Space. Core resolves the destination and checks its export list on
the exact connection used for dispatch, without retaining a registry lock over IO.

## Export and receive

`NodeDescription.details.pluginAccess` is an optional, typed extension:

```json
{
  "requests": ["/plugin/camera/source/list"],
  "events": ["/plugin/source/changed"]
}
```

Absent exports mean no Plugin-callable routes. Each exact target must also occur
in `NodeDescription.routes`; malformed, duplicate or unadvertised entries reject
the description. Targets start with `/plugin/`; they contain nonempty path
segments made of letters, digits, dot, dash or underscore, excluding `.` and `..`.
App declarations do not grant Plugin access, and Plugin exports do not grant App
access. Any bound Plugin may use this mechanism; the destination owns business
authorization using the Core-authenticated caller.

Core forwards the business request as `{version,payload}` at `target`, with separate
transport control:

```json
{
  "kind": "pluginCall",
  "spaceId": "fictional-home",
  "from": {"nodeType": "camera", "nodeId": "fictional-camera"},
  "timeoutMs": 10000
}
```

The receiver SDK verifies Space, message kind and export membership before runtime
admission. `timeoutMs` is present only for requests. The SDK supplies the context
separately to `on_plugin_request` or `on_plugin_event`; business payloads are not
augmented with identity fields. Both callbacks belong to an admitted runtime generation.

## Requests and failures

`timeoutMs` defaults to 30000 and permits 1–60000 milliseconds. SDK queueing counts
toward its request deadline. Nested runtime calls inherit the remaining local
deadline. Core queueing and delivery share its remaining budget; a downstream hop
does not reset it. Expired queued work is not started. No clock synchronization
between Plugin Hosts is required.

The destination SDK returns `{version,result}` using the existing Full
`node-operation-result.v1` contract. Core does not interpret the business result;
success, partial, error and cancelled remain destination-owned values. The caller
SDK checks the response version and shape. Transport errors retain their original
code, message and optional details, including Plugin-defined codes.

Core allocates independent downstream correlation IDs. Concurrent senders and
out-of-order replies cannot share response waiters. Late replies are discarded.
Timeout, cancellation or connection loss does not establish that a mutation did
not execute; neither Core nor the SDK automatically retries. Caller cancellation
does not free an SDK request slot while its transport request remains pending.

## Events and lifecycle

`emit` succeeds on local queue admission only. Events have no remote receipt,
business response, persistence, automatic retry or reconnect replay. No ordering
is promised across concurrent sends or reconnections. Business protocols that
need confirmation use requests; state synchronization uses a fresh authoritative
read after reconnect. Business subscriptions and their renewal belong to Plugins,
not the Core router.

The receiver SDK has a separate bounded event lane, with count and byte limits,
so pending RPC handlers cannot prevent event callbacks. Connection loss discards
queued events; already admitted work may finish under its original runtime guard.
Generation changes reject queued old work, and runtime forced cancellation fences
active callbacks. Message-size limits and bounded transport queues also apply.

This layer supplies control-plane JSON communication. It does not transport video
frames or replace existing media and Artifact protocols.
