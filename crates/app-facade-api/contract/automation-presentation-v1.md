# Automation UI presentation reads

All four targets use the normal App Facade envelope and Space read authorization.
They are UI entrypoints, not Agent/CLI/MCP tools. Inputs reject unknown fields.
Full and Essential use the same business read scope; UI requests Full.

| Target | Input | Output and read boundary |
| --- | --- | --- |
| `/app/presentation/automation/names/get` | `{ids}`: 1–100 selected, trimmed/deduplicated IDs | `{automations: {id: label}}`. Missing IDs omitted. Selected definition metadata only; no source downloads, parsing, state, history or graph reads. |
| `/app/presentation/automation/list` | `{type: "RECURRING" \| "TASK_FLOW"}` | `{automations}`: current-type regular user programs, with labels, runnable/health/expiry metadata, enabled/running/blocked, last-run `timestamp`, status timestamps and tags. No temporary-program, Group, Widget, snapshot, graph or authored-source expansion. |
| `/app/presentation/automation/overview/get` | `{automationId}`: nonblank, trimmed | `{id,name,type,enabled,runnable,corrupted,blocked,widgets}`. Selected metadata, status, cloud quota display state and owned Widget configuration/current values. No source, graph traversal, running history or snapshots. Missing definition is not-found. |
| `/app/presentation/automation/placements/get` | `{automationId}`: nonblank, trimmed | `{canvases:[{id,name,containsAutomation}]}`. Reads owned Widgets, Widget-to-Card relations and canvas metadata/card IDs; never loads Card bodies. Retains existing default-canvas initialization when the Space has none. |

Labels use saved preferredName, then the script-derived saved name, then the
existing unknown-label fallback. Metadata reads never require source storage to
be available. Names and overview permit generated-program inspection within the
authorized Space; this does not widen user-program mutation Facades. Group
management remains in the Group domain.

Core filters the regular definition table by type in SQL and uses selected state
reads. Cloud definition storage is partitioned by Space, so list reads that
partition's stored metadata and filters by type before loading state. It does not
hydrate source bodies or read the temporary/Group stores. This is not a promise
of a type-specific physical index. Billing display and lifecycle state retain
their existing authorities; enabled, runnable and running are independent.

List Full timestamps are milliseconds. Essential renders explicit timestamp
fields in the Space's Home timezone. Widget values retain their existing Widget
DTO semantics; their numbers are not interpreted as timestamps.

The frontend loads only the active list type, caches visited types within the
active Space, applies realtime events to loaded rows, and invalidates inactive
caches on incomplete updates or reconnect. Unloaded tab counts are unknown,
not zero. Overview and placements are separate on-demand reads. Snapshot panels
continue using `/app/automation/snapshot/list` only when opened.

Automation detail includes `runnable` from its already-loaded definition; Group
entries omit it. Automation status includes `corrupted` from the existing status
and temporary `expireAt` from creation/last-run timestamps. Ordinary configuration
editor initialization supplies the metadata it already loaded, so subsequent
runtime hydration only needs status.
