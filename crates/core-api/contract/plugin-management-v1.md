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

## Business contract ownership and discovery

Plugin-owned JSON contracts live with their implementations, independently of
Foundation packages. The official Camera, Matter and Mac sources are
`mhome-ai/integration`, under `plugins/<id>/contract.json`. Their business payloads,
operation identifiers and fixtures are not published in Foundation. LLM and Storage
platform backend contracts and canonical capability schemas remain platform-owned.

A signed release can reference business contracts under
`nodeMetadata.plugin.businessContracts`: each reference contains `id`, `version`
(contract SemVer), `sha256` (exact JSON bytes), package-relative `path`, and implemented
`operations`. Native packages carry the referenced JSON; firmware can embed the
reference and retain the pinned JSON in its source distribution.

The authenticated Node description may advertise `details.businessContracts`, with
`id`, `version`, `sha256` and its actual supported `operations`. This is descriptive,
not an authorization grant. `routes` and `appAccess` still control availability and
permission. The Plugin detail Facade projects public `api.businessContracts` and
`api.appAccess` from that session, never arbitrary runtime details. `api: null`
means offline/loading/unavailable; a null `businessContracts` means this older
implementation has no advertisement. Neither is an empty supported set.

Clients pin the business contract's identity and compatible major, and check
operation availability separately. An implementation may support a subset. A
schema/document move does not change wire version, payload shape or permissions.
Consumers tolerate additive response fields. New requests must not send fields
or operations that the selected implementation has not advertised support for;
unknown enum variants require an explicit fallback or a new major. Legacy strict
readers do not become compatible simply because this rule was documented.
