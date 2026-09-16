# Hub-owned System facade

A System is the Hub's LAN, plus its Cloud bridge and current Space clients.
The Hub Core is the aggregation point. Its local Host supplies mDNS discovery;
Core reads each discovered Host directly over HTTP. Host does not aggregate peers.
Lion dispatches these exact targets to the selected Hub for a local-mode Space.
Client daemon and native UI bridges do not aggregate this view or perform its maintenance.

## Authority

All current Space members can query and maintain the System. Core verifies
membership; Lion applies the same member boundary (READ for observations,
WRITE for plan/start/restart). Maintenance changes shared Host services and can
affect multiple Spaces; this is intentional. Host's LAN control API retains
its existing trusted-LAN boundary. A hostId selects a Host, not an identity proof.
Caller-provided URLs, commands, paths and arbitrary proxy actions are not supported.

Only commissioned Node identities for the current tenant and Space appear.
An offline commissioned identity remains visible. No service instance discovery
RPC is added. Host inspection's multi-Space runtime diagnostics never cross the
facade: explicit projections expose only machine facts, components and operation
receipts. Hub lifecycle, Hub–Cloud connection, Node–Hub connection and runtime
health are separate fields. Reconnecting a Node does not refresh an old report.

## Endpoints

| Target | Input | Output |
| --- | --- | --- |
| `/app/system/inventory/get` | empty | Current LAN Host observations plus current Space commissioned instances |
| `/app/system/clients/get` | empty | Current Space live client sessions |
| `/app/system/host/metrics/get` | hostId | Independently timed metrics observation |
| `/app/system/host/inspect` | hostId | Machine installation capabilities, components, safe service status, recent operations |
| `/app/system/host/plan` | hostId, components, optional all | Signed-catalog installation plan receipt |
| `/app/system/host/start` | hostId, planId | Persistent installation operation, idempotent by planId |
| `/app/system/host/operation/get` | hostId, operationId | Persistent operation status |
| `/app/system/host/restart` | hostId, component, operationId | Persistent Core/Node restart receipt, idempotent by caller-generated 32-hex operationId |

Restart is queued on Host before execution. The caller retains its operationId
across response loss, then polls or retries that same id. No automatic POST retry
occurs in Core. Host rejects reuse of an id with a different component or plan.
Core failure recovery, including access while the Hub is down, is outside this
implementation. After a planned restart, polling resumes when the Hub reconnects.

## Observation semantics and cost

`status` is `ok`, `unavailable`, `unsupported`, or `notQueried`. `data` and
`observedAtMs` are null until the first successful observation. Failed refreshes
preserve successful data and its timestamp and mark it stale. `lastAttemptAtMs`
tracks actual attempts, not render time. An unavailable service list is not an
empty list. Source is `local`, `mdns`, or `commissioned` (reference only).

Inventory is single-flight with a 3-second cache, four in-flight resource queries,
an 8-second query budget after discovery, local priority and rotating remote order.
Each query races at most two advertised IP addresses with a 3-second budget;
info and services complete independently. Discovery has a separate 3-second cache
and deadline. Metrics are queried on demand and coalesced per Host. Eight concurrent
resource queries bound combined inventory/metrics work. Responses are capped at
1 MiB, redirects/proxies are disabled, and returned Host identities are checked.
No permanent topology scanner, binding-expiry write or runtime reconciliation is
triggered by observation. Frontend owns graph layout and polls only while visible.

The old `/app/topology/get` and `/app/topology/changed` routes are retired.
Device Topology remains a separate contract.

## Build and validation

Protocol changes originate in Foundation and are published to crates.io and npm.
Core and Baycat resolve the published Rust packages, including independent Android
and Messaging roots. Pallas resolves the matching published npm protocol package.
Consumers do not carry copied protocol sources or local protocol tarballs.
