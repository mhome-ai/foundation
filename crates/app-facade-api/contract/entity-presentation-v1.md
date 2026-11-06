# Entity UI presentation

## Selected reference metadata

`/app/presentation/entity/meta/get` accepts `{entityType, ids}` under the normal
Facade envelope. It is a Space-read UI operation, routed by Space mode, not an
Agent/MCP catalog entry. IDs are nonblank, trimmed, deduplicated in request order
and bounded to 100 input entries. Empty input and unknown fields are rejected.
The response is `{items: [{requestedId, found, resource? , reason?}]}`. A found
resource contains only `id`, `entityType`, `displayName`, and `group`. A missing,
deleted or wrong-type reference returns `found: false, reason: "not_found"`.

Automation uses definition metadata; Widget uses its own definition; other
entities use their stored Twin identity. `g:` IDs read the Group definition and
check its member type (an empty legacy member type accepts the requested type).
No Group members are resolved. All reads use authenticated tenant and Space.
No source refresh, room lookup, runtime status, history, card/relationship read or
script expansion is performed. Missing display names remain empty; the UI owns
its fallback label. Both response detail modes preserve these business fields.

This is a selected-reference resolver, not a replacement for the domain catalogs
of special entities such as AppClient or Location. It does not enumerate them.

## Entity renderer aggregate

`/app/presentation/entity/get` is a read-only App Facade for the UI. Its direct
input is `{entityType, entityId}`; tenant, Space and caller come from authenticated
context. It requires Space read access. It is not an Agent or MCP catalog entry.

The successful data object contains `entityEntry`, nullable `entityCard`, and
`twinAutomations`. The entry and card reuse the existing UI renderer models;
the schema owns this boundary without duplicating their nested widget/runtime
contracts. A missing, deleted or differently typed Twin is not found. Device
presentation reads reuse the device manager; other Twin entities reuse the general
manager. Ordinary Entity business detail and Group detail retain their own APIs.

Device card assembly is best effort. An absent card is `null`, with an empty
association list. Association-read failures retain the existing execution policy:
local Core returns an empty list; cloud propagates the error. No writes or repairs
are performed. Current Twin states still have their separate read/subscription.

UI uses `control: {mode: "direct", responseDetail: "full"}` and receives the shared
`{status: "success", data: ...}` envelope. The generic essential projection returns
`{status: "success", data: ...}` with the same business fields rendered as YAML;
there is no domain field reduction. This is not the compact business detail API.
