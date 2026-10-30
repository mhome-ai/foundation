# mhome-core-api

Canonical internal wire protocol for MeowCore, host services, Nodes, and
provider runtimes. Public `/app/*` contracts belong to
`mhome-app-facade-api`; this crate owns internal MWS, authentication, LLM,
Messaging normalization, Node runtime, and Storage contracts.

Plugin management uses the [generic transport contract](contract/plugin-management-v1.md).
This package distributes shared Node wire contracts, including Matter and Camera
management schemas; each Plugin owns their execution. App consumers use the
opaque Plugin Facade envelope, and Core does not aggregate these inventories.

`messaging::IngressRedeem` carries a parsed identity code and a dedicated
`IdentityRedemptionContext`: event ID, receiving address, sender and optional
conversation display name. Redemption is independent of normalized message
content and its schema version. The receiver validates the context and authorizes
the registered account before consuming the code. External protocol 23 requires
Host and Core to use the dedicated context; cloud ingress must be upgraded with
the local receiver. There is no full-message redemption compatibility path. The shared wire example is
[`fixtures/messaging-redemption.json`](fixtures/messaging-redemption.json).

The additive [`client_identity`](contract/client-identity.md) contract separates
User and Space Client identity, installation ID and enrollment ID. Runtime ingress
has not yet cut over to this contract; operation authorization remains separate.
The additive [`space_access`](contract/space-access.md) contract defines the
READ/WRITE/MANAGE hierarchy and its pure User/Space Client decision. It does not
change runtime admission until adapters and operation classifications adopt it.

MWS uses standard WebSocket Ping/Pong control frames for connection liveness.
There are no application `pi`/`po` messages or auth `heartbeatInterval` field.
The transport owns heartbeat timing and reports authenticated local connection
activity through `RefreshLocalAppClient`; closing the transport removes that
connection through `CleanupLocalAppClient`.

`NodeDescription.appAccess` declares exact runtime targets and their required
Space READ/WRITE/MANAGE level. Core reads it over the authenticated Node connection
and checks App-originated calls before dispatch. Every `/app/` and
`/interaction-flow/` route in `routes` must declare access; undeclared targets are
not App-callable. Internal service calls keep their existing trusted boundary.
This is business authorization, independent of operating-system permissions.

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

Final business input uses the fixed `complete` route. Its submission identity
contains the Flow ID, completion command ID, and an owner derived by Core from
the authenticated Space and principal. The Node persists this input before
installation in its own business store. Independent `submission/status` and
`submission/retry` routes work after source-session or Core restart; they still
require the exact bound Node and current access checks. Status never installs.
Closing the source session does not undo accepted data. Business mutations and
applied receipts must commit atomically; a later retry uses current business
credentials rather than replaying the old final input.

Storage separates backing-filesystem capacity from Storage-owned logical
usage. Namespace is an internal protocol term; user-facing clients present it
as a Folder.

## Lossless external boundaries (core-api 1.19.0)

External protocol **20** adopts Artifact lifecycle requests and responses (`std`, `tmp`,
`man`) and v2 resource URIs. Host and Core must agree on this version before exchanging
artifact messages; native memory/disk descriptors retain their `temporary` boolean.

External protocol **19** adds `metadataOnly` to `AuthorizeArtifactRead` for HEAD
and other metadata probes. Such responses contain metadata without a path or
inline content and do not renew temporary retention. Attachment upload clients
must concurrently receive an early error response while sending the body, so
an expired grant is preserved as a domain error instead of a broken connection.

External protocol **18** adds bounded, connection-scoped attachments for large
memory-backed Artifact content. Artifact commit requests and open/read responses
use an empty `dataBase64` marker followed by a big-endian u32 length and that many
base64 bytes. The body is bounded by the existing 50 MiB per-file limit before base64 encoding;
ordinary JSON frames retain the 16 MiB limit. No upload session or disk spill is
created for this transfer.

The protocol returns `HttpPayloadResponse { statusCode, body }` from
all HTTP handler RPCs. A completed HTTP response (including 4xx/5xx) is an RPC
success; RPC errors describe failures to execute the call, not HTTP status.
Hosts preserve status and JSON body, never infer them from error messages.
`ExternalCoreError` preserves domain `code`, `message` and optional `details`,
with conversions to/from the shared client `ErrorResponse`.

The transport-neutral `webhook` module defines the common route, body limit and
JSON-versus-UTF-8-text decoding. Hosts own network listeners and method admission;
Core owns dispatch and execution. Upgrade service shell and Core together.

## Host LAN observations (introduced in core-api 1.18.0)

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

## Host management authorization

`host::access` owns cloud-issued short User tokens, fixed Admin/Space modes,
signed challenge/claim/receipt types and permissions. `host::auth` owns Host public
keys and the cloud signing-key directory. See
[`contract/host-management-auth.md`](contract/host-management-auth.md).
Business Client–Core identities are a separate protocol.

## Native Client Host management

`host::management` defines the generic runtime request/action and response envelope.
Runtime implementations supply their component ID type. The matching language-neutral
contract is `contract/host-management-v1.json`; request validation uses
`schema/host-management-request.v1.schema.json`.

Desktop/CLI use Clientd IPC `host.list` and `host.runtime`; mobile Clients expose the
same operations through `hostList` / `hostRuntime`. None accepts a Space ID. Clients
discover `_mhome-host._tcp`, verify reply Host IDs, and unwrap the HTTP runtime envelope.
Install/restart submissions are not automatically replayed after transport failure.
First claim is direct LAN only; daily User management can use the current Core as a token-forwarding proxy.

## Operation timeouts

`manifest/operation-timeouts.v1.json` is the code-owned catalog of shared operation
budgets, ingress selectors and caller reply-wait profiles. It has no runtime
configuration or database dependency. Edit the catalog and run
`npm run generate:operation-timeouts`; Rust consumes generated constants and Lion
loads the same packaged JSON once at startup. Both replay the packaged
`fixtures/operation-timeouts.conformance.json`. Java bindings reject unknown policy
IDs instead of silently choosing a different limit.

An execution budget and a reply wait are different: reply grace permits transfer
of a completed result or timeout error; it never grants the callee more execution
time. Existing domain-owned lifetimes (streams, subscriptions, long batch jobs)
and implementation-specific step timeouts are not replaced by a default request
budget. New shared operations need an explicit catalog entry and selector tests.
`hubMigration` gives an accepted migration worker up to ten minutes, including drain
and transfer. Starting a migration and querying its status remain ordinary requests.
Expiry fails the attempt; it does not prove that source requests drained or reopen
the Space. Explicit retries replace uncommitted snapshots and finish committed ownership.

`execution_budget::Budgeted<T>` adds optional `executionBudget: { remainingMs }`
metadata alongside an object envelope's existing fields. Bridge uses it on MWS
requests; Host IPC uses it alongside `input` in `HandleCoreInput`. It is not an
App Facade `control` field or part of the business payload. Values are integral
milliseconds in `0..=2147483647`; zero is expired. Missing metadata retains local
policy. This additive metadata does not change external IPC protocol 23; both
ends need the new implementation for remaining-budget propagation.

Each process anchors received time to its monotonic clock, caps it by local policy,
and passes the remaining budget to children. Sender queueing/upload, receiver
queueing/download, attempts and backoff consume that same deadline. Round down
when emitting milliseconds. Durable Lion peer relays also deduct queue age using
the existing peer message timestamp/clock convention. Network transit and clock
skew are not a strict global stop guarantee; the caller keeps its own deadline.
Do not put a refreshed full duration around each retry.

Timeout means that the caller stopped waiting and cancelled owned asynchronous
work where supported. It does not roll back completed writes, stop arbitrary
blocking code, or prove that detached work drained. Request-owned spawned tasks
must explicitly carry the captured deadline. Responses and independent background
jobs have their own lifetime. Migration freezing/drain remains a separate feature.
