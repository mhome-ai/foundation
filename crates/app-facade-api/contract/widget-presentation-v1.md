# Widget UI presentation reads

These are UI App Facades with the standard Full/Essential response and Space read
policy. They are not Agent/CLI/MCP tools. Inputs reject unknown fields.

| Target | Input | Business output and read boundary |
| --- | --- | --- |
| `/app/presentation/widget/list` | Required `{type: "data" \| "button" \| "circadian"}` | `{items, automationNames}`. Reads only that type's Widget table. Global standalone Widgets, defaults and Automation-owned parameters are included. Active card interiors, Twin and weather internals are excluded. Pending Data, Button and Circadian Widgets eligible for manual deletion are included for cleanup, regardless of their usual visibility. Owner labels use one selected metadata batch, only for returned Automation owners. No script, relation, history or all-type reads. |
| `/app/presentation/widget/names/get` | `{ids}`: 1–100 selected, trimmed, deduplicated IDs | `{names: {id: displayName}}`. Missing/blank names omitted. Only selected Widget rows; no legacy `name` fallback. |
| `/app/presentation/widget/defaults/get` | `{}` | `{items, automationNames:{}}`. Preserves the existing default Widget lookup and lazy initialization, then reads the selected Data Widgets. |

Rows carry ID, revision, displayName, description, source/baseId, global, tag,
current value, and type-specific control configuration. `pendingDeletion: true`
identifies a Widget whose deletion can be completed through the delete Facade. Circadian rows contain
only `curve`, `curvePreview` (active curve's UI points), current/next values and
`nextTimestamp`; they omit complete schedules, location and internal state.
Complete configuration uses `/app/widget/detail/get`; runtime uses
`/app/widget/value/get`, both selected-ID reads. Full timestamps are milliseconds;
Essential renders explicit timestamps and typed Timestamp values in Home timezone.

`/app/widget/list` remains the shared summary catalog. Optional `type` limits the
physical Widget-table read; optional `excludeSources` filters known sources
(`widget`, `automation`, `special`, `twin`, `weather`). Omitted filters preserve
the existing global supported-type catalog. It never returns control values or
full definitions. Dashboard selection excludes Automation-owned parameters and
adds selected IDs directly without definition/value reads.

The UI loads only the active type, caches visited tabs within the Space and never
fetches inactive counts. Automation parameters are grouped by owning Automation;
users can operate them and open the owner but cannot edit/delete their definition.
Realtime changes update loaded rows, deletes remove them, and reconnect or
incomplete presentation changes refresh only the relevant active tab. Space
switches clear caches and reject late responses. All public Widget request
Controllers, including reads and writes, are removed. Writes use the shared
`/app/widget/*` Facades described in `widget-writes-v1.md`.
`/widget/reportUpdate` remains a push protocol; internal Automation dispatch
remains separate from public request routing.
