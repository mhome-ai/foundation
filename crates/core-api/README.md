# mhome-core-api

Canonical internal wire protocol for MeowCore, host services, Nodes, and
provider runtimes. Public `/app/*` contracts belong to
`mhome-app-facade-api`; this crate owns internal MWS, authentication, LLM,
Messaging normalization, Node runtime, and Storage contracts.

MWS uses standard WebSocket Ping/Pong control frames for connection liveness.
There are no application `pi`/`po` messages or auth `heartbeatInterval` field.
The transport owns heartbeat timing and reports authenticated local connection
activity through `RefreshLocalAppClient`; closing the transport removes that
connection through `CleanupLocalAppClient`.

`interaction_flow` defines the serializable Core-to-Node flow definition and
handler protocol. Definitions name logical operations only; MeowCore chooses
and pins the Node instance and fixed transport routes for each session.

The Node owns the source session returned with a definition. Core closes that
session through the fixed close route after completion, cancellation, expiry,
or a failed start. Close is idempotent, and Nodes must also expire abandoned
source sessions so a Core restart cannot leak them. Execute operations are
idempotent within a source session: replaying the same `operationId` and
request returns the original result, while reusing the ID for a different
request is rejected.

Storage separates backing-filesystem capacity from Storage-owned logical
usage. Namespace is an internal protocol term; user-facing clients present it
as a Folder.

## Host LAN observations (core-api 1.18.0)

The Host owns `host::network::HostNetworkSnapshot`, exposed through local IPC
`GET /internal/network`. It samples eligible private IPv4 interfaces at startup
and every 15 seconds, preferring `192.168/16`, then `10/8`, then `172.16/12`.
Within the best priority it retains a still-valid address, otherwise picks the
numerically smallest address. This is a deterministic common LAN policy, not
Ethernet/Wi-Fi classification or proof of reachability to every destination.

The response is `{ "lanIpv4": "192.168.1.2", "observedAtMs": 123456789 }`;
`lanIpv4: null` means unavailable. Consumers reject observations older than
45 seconds, future observations, and non-private IPv4. No loopback fallback.
The query does not enumerate interfaces. Existing multicast discovery still
uses its own complete interface inventory, not this single preferred address.

External protocol 16 changes `UpdateCallbackBaseRequest` to `{ network, port }`.
The service supplies the actual bound HTTP port and the unchanged Host
observation. Core derives the origin instead of trusting an independent URL.
Before the service reports, the sidecar has no callback origin. Freshness-only
updates do not trigger subscription reconciliation. Shell and runtime must be
upgraded together; local IPC is distinct from advertised callback URLs.

## Playground Provider (core-api 1.10.0)

External protocol 15 removes the dedicated Playground webhook method and payload.
Playground Provider delivers events through the standard subscription webhook,
using `GeneralWebhookPayload`. Update the Core service shell and runtime together;
mixed external protocol versions are not supported.

## Stable recipients (core-api 1.8.0)

`RecipientId` provides canonical Messaging (`m:p:` / `m:g:`) and Node (`n:`)
addresses. See [the contract](contract/recipient-id-v1.md) and the packaged
cross-language [conformance vectors](fixtures/recipient-id.conformance.json).
This is additive; the external Host transport protocol remains version 14.
Consumers must explicitly migrate their identities after this release is available.

## Host delivery (core-api 1.7.1)

Core resolves logical recipients; the Host executes a concrete App connection,
Node connection or Messaging destination. Delivery responses distinguish queue or
provider acceptance, intentional skips, definite failure and unknown completion. Acceptance is not
a user-read receipt. Connection close has a separate typed control request.

External protocol 14 replaces effect notifications with request/completion events.
ServiceCoreOutput contains only its response and rejects the old effects field.
Messaging returns the same typed outcome, not a delivered boolean. Skipped means
handled without a send and must not be reported as provider acceptance.
There is no old-protocol compatibility path. Consumers must upgrade together;
version 1.7.1 must be published before updating registry-backed consumer locks.

## Host contracts (core-api 1.11.0)

`core_api::host` is the single owner of Host hardware information, installed
service inventory, system/service metrics, and OS permission declarations and
observations. It replaces the independent `mhome-host-api` crate. App Facade
reuses these types and owns its public routing and observation envelopes. Importing
this module does not require a running Core or a Space.

Host/Client producers and readers must use these DTOs rather than redeclare
matching JSON structures. Platform collectors may keep private sampling structs;
only the canonical DTOs cross HTTP/IPC boundaries. The permissions contract is in
`contract/host-permissions.md`; runtime permission handlers remain a separate
implementation step. The existing Core external runtime protocol stays at 15.

## Offline Host authorization (core-api 1.15.0)

`host::auth` is the language-neutral Client–Host authorization payload contract.
See [`contract/host-offline-auth.md`](contract/host-offline-auth.md). This crate
does not implement signing, key storage, or transport.

## Native Client Host management

`host::management` defines the generic runtime request/action and response envelope.
Runtime implementations supply their component ID type. The matching language-neutral
contract is `contract/host-management-v1.json`; request validation uses
`schema/host-management-request.v1.schema.json`.

Desktop/CLI use Clientd IPC `host.list` and `host.runtime`; mobile Clients expose the
same operations through `hostList` / `hostRuntime`. None accepts a Space ID. Clients
discover `_mhome-host._tcp`, verify reply Host IDs, and unwrap the HTTP runtime envelope.
Install/restart submissions are not automatically replayed after transport failure.
Existing LAN access semantics remain; this release does not add a trust/pairing protocol.
