# Entity UI presentation

`/app/entity/presentation/get` is a read-only App Facade for the UI. Its direct
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
