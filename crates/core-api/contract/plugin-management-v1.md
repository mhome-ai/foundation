# Plugin management transport

Business management runs in the selected Plugin. Core authenticates the caller,
resolves the exact current-Space binding, enforces the connected Node's declared
`appAccess`, and forwards `/app/plugin/{nodeType}/{route}` to
`/{nodeType}/app/{route}`. Input is `{nodeId, version, payload}`; the Node receives
`{version, payload}`. The App Facade's successful data is
`{nodeId, nodeType, version, data}`. Consumers validate identity and version before
reading the business `data`. Business payloads are never Integration inventories
or Twin entity IDs merely because a field is named `devices`.

## Preparation

A Plugin can implement a read-only `/{nodeType}/app/{route}/prepare` handler with
its own `appAccess` declaration at least as restrictive as execution. The request
uses the same version and original payload. Its runtime response data is
`{preview:{title,message,details},payload:{...}}`. `payload` is the concrete
business command with any revision or resource-set preconditions; `details` is
safe public confirmation content. Neither may contain credentials.

Core stores both but exposes only the public preview, adding the trusted Node ID
to its details. Commit revalidates authority and binding, repeats preparation,
and rejects changed previews or execution payloads. It then replaces only the
original request's payload. Target, Node ID, version and approval identity remain
Core-owned. The Plugin checks its preconditions atomically with the mutation.
There is no guarantee of exactly-once external effects. Existing platform-owned
LLM and AudioBridge previews retain their adapters.

## Management events

A bound Node may publish `/node/app/event` with
`{version:"v1",event:"inventory/changed",data:{...}}`. Event names consist of
nonempty lowercase alphanumeric, underscore or hyphen segments separated by `/`,
up to 256 bytes. Core derives tenant, Space, Node ID and Node type from the
authenticated session and publishes `/app/plugin/event` to that Space with
`{nodeId,nodeType,version,event,data}`. A Node cannot supply another publisher's
identity or choose a platform event target. Consumers filter publisher identity
and Space. This is an observation; capability subscriptions retain their separate
protocol and delivery ledger.

## Matter

The Matter Plugin owns `status`, `commissionable/list`, `fabric/get`,
`fabric/node/detail`, `inventory/get`, `onboarding/start`, `onboarding/input`,
`onboarding/status` and `fabric/node/remove`. Read routes require Space read;
start, input and removal require Space write. There is no `/app/matter/*` Facade.
The management inventory and detail payload schema is
`schema/matter-plugin-management.v1.schema.json`.

`status` returns runtime information or `runtimeError`, plus trusted `spaceId`.
Binding and offline observations come from `/app/plugin/enabled/list`, which can
answer even when the Plugin is unavailable. Commissioning retains operation ID,
session ID and expected revision semantics. Consumers pin the Node ID throughout
the operation and its durable continuation; closing a UI does not cancel it.
Discovery publishes `commissionable/report` through the generic event channel.

The separate Integration `/matter/device/list` response remains
`{"ok":true,"data":[...]}`. It enumerates source entities and is not a management
list. ESP implementations must implement and advertise only supported routes;
they must not dual-write alternate array/object envelopes.

These contracts continue to ship with Foundation. Plugin execution ownership does
not introduce a new package publication system or move provider schema sources.
