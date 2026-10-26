# mhome-app-facade-api

Canonical public protocol shared by MeowLink clients, MeowCore, the CLI,
Agents, and cloud adapters.

The package also carries the explicitly private `/local/*` native-daemon
extension. `/app/*` is portable and may be routed according to the public
routing manifest. `/local/*` is an exact allowlist for the current MeowLink
native client process and must never be forwarded to cloud, Hub, or Host
connections. Hub `/app/*` calls use `FacadeCall`, whether delivered directly
or forwarded by authenticated Cloud ingress. The separate Hub `/local/*`
protocol carries domain input and never uses the public envelope.

Every `/app/*` request uses the strict `FacadeCall` envelope. Invocation
metadata lives in `control`; the domain request lives in `input`. The crate
owns public targets, request/response/event schemas, and stable operation
identity. Core-to-Node routes and provider runtime models belong to
`mhome-core-api` instead.

`interaction_flow` exposes only the materialized current-step view and session
commands. Handler operations, the complete step graph, and Node routing
identity remain internal to MeowCore.

Core owns defaults, resolver results, and read-only values. Clients may write
only editable inputs. Password plaintext is accepted as input but is never
projected back; `hasValue` lets a client render back navigation without
recovering the secret. Each current-step resolver projection reports whether
its complete referenced value is available and which visible inputs can make
it stale, so clients can resolve it reactively without learning handlers or
the hidden graph.

`topology` exposes the Hub-owned Space network projection. Stable Host, Hub,
Node, AppClient, and Cloud identities are separate from current transport
sessions. Durable relations remain present while disconnected and carry an
explicit observation state so clients can distinguish expected topology from
the currently observed path.

`artifact` exposes the MeowLink-facing `/app/artifact/resolve` operation. Its
domain input and output reuse `mhome-artifact-api`; Node runtimes use the
separate transport-neutral `/artifact/*` targets and never enter the App
Facade.

`pod` (1.34.0) defines native-Client pod discovery and commissioning
(`/local/pod/*`), the shared BLE advertising format and the pod credential
protocol with Lion. Sessions are owned through a renewed lease, and rejected
requests carry a stable `details.reason`. See
[the contract](contract/pod-commissioning-v1.md). Targets, BLE constants, session
timings and request reasons are in `manifest/pod-targets.v1.json`; snapshots use
`schema/pod-discovery.v1.schema.json` and `schema/pod-commission.v1.schema.json`;
User Pod enrollment uses a one-time login approval; activation is confirmed on the paired device.

`/app/pod/list` combines the current User's personal registrations with the current
Space's Space-owned Pods, including offline entries. `/app/pod/rename` updates a
personal registration name. See [Pod directory](contract/pod-directory-v1.md). `/app/host/list` combines Host registrations with current observations and details.
`/app/host/directory/list` is the historical registration input used by native
list composition; it does not grant management access. See the
[Host directory contract](contract/host-directory-v1.md). Native commissioning
uses the existing device protocols; the Agent resumes only after verifying the
resulting registration. Host completion identifies both `hostId` and `env`.

`host_provision` (1.34.0) defines BLE Wi-Fi provisioning of embedded Hosts
(`/local/host/provision/*`) on the same BLE layer; discovery candidates carry
`kind`. A completed session carries the one-time `claimToken` that the first
claim of that Host requires. See [the contract](contract/host-provisioning-v1.md), the targets in
`manifest/host-provision-targets.v1.json` and
`schema/host-provision.v1.schema.json`.

## Routing contract

`manifest/routing.v1.json` is the shared business execution policy for the
new `/app/*` facade. Ordinary operations follow the active Space mode. Only
exceptional domains declare a fixed Cloud, Hub, or request-placement executor.
Cloud-capable User Clients may enter through Cloud for registered public control
operations. Lion resolves the same execution rules before converting domain input,
and forwards complete Core-owned calls through the authenticated Bridge. Runtime
connection state, ingress restrictions, authorization, and surface exposure remain
separate concerns. A transport change never changes business ownership or replays
a request whose outcome is unknown.

The checked-in Rust table is generated from that manifest, and Lion and non-Rust clients
consume the same manifest from the npm protocol package. Local Spaces execute
Host inventory and management on the selected Hub through `/app/system/hosts/*`.
A native Client remains the path when no local Hub is selected. Lion's legacy
controller annotations and non-`/app/*` protocols are outside this contract.

### Plugin catalog

`/app/plugin/catalog/list` reads publisher-signed platform indexes under
`https://install.mhome.ai/plugins/stable/<platform>/catalog.bundle.json`.
Cloud serves this browsing view without a connected Hub. Authentication and
Space membership still apply. Core exposes the same response for local callers.
The response schema is `schema/plugin-catalog.v1.schema.json`; `platformErrors`
reports unavailable indexes while preserving results from other platforms.

Foundation defines contracts, not the list of Plugins. Plugin source owns its
definition; releases publish it with each immutable per-version manifest.
`componentId` joins Host inventory, and `nodeType` joins candidates and bindings.
Catalog availability is a browsing hint. Binding uses `/node/metadata` from the
actual running Node, verifies publisher evidence, and persists that definition
with the binding. A cached latest catalog never overrides installed facts.

## Shared messaging codes (1.10)

Code creation may omit bot/account and group/surface identifiers. The receiving authenticated provider determines the bot; shared candidates are confirmed in the App. The existing actor link claim status/confirm/event endpoints carry both kinds, distinguished by `kind` (`link` by default, or `bind`). Bind code responses include `challengeId`; completed bind claims omit `linkId`. Private link completes on receipt. A code is single-use within its provider and service deployment, never a cross-provider or cross-Hub credential.

## Space runtime observations

`/app/system/instances/get` reads Space instances; `/app/system/clients/get` reads
Client sessions. Host inventory and operations use `/app/system/hosts/get` and
`/app/system/hosts/runtime`, forwarding cloud-issued User tokens through the
current Core. First claim is direct LAN only. See [SYSTEM.md](SYSTEM.md).

Business operation permissions are declared once in [the authorization manifest](manifest/authorization.v1.json), consumed by Core and Lion. See [business authorization](contract/authorization-v1.md).

## Matter management reads

`/app/matter/device/list` is the management inventory, grouped by Fabric node. It is distinct from the Plugin's Integration enumeration `/matter/device/list`. Its `sourceDeviceId` values identify Plugin projections, not Twin entities. Nodes without projected devices remain visible; `membershipComplete`, `inventoryComplete`, per-node completeness and `errors` describe partial observations. Lists omit endpoints and attribute payloads.

`/app/matter/fabric/node/detail` accepts 1–20 unique `matterNodeIds`, returning an ordered item result or error for each ID. The request uses string IDs without numeric coercion. See `schema/matter-management.v1.schema.json`. All `/app/matter/` operations execute through the Space's Hub irrespective of scope execution mode. Node removal is a manual UI/CLI capability, not a delegated Agent/MCP tool.
