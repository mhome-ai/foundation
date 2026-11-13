# Automation UI presentation

All targets use the normal App Facade envelope. Reads require Space read authority; editor and mutation targets require Space write authority.
They are UI entrypoints, not Agent/CLI/MCP tools. Inputs reject unknown fields.
Full and Essential use the same business read scope; UI requests Full.

| Target | Input | Output and read boundary |
| --- | --- | --- |
| `/app/presentation/automation/names/get` | `{ids}`: 1–100 selected, trimmed/deduplicated IDs | `{automations: {id: label}}`. Missing IDs omitted. Selected definition metadata only; no source downloads, parsing, state, history or graph reads. |
| `/app/presentation/automation/list` | `{type: "RECURRING" \| "TASK_FLOW"}` | `{automations}`: current-type regular user programs, with labels, runnable/health/expiry metadata, enabled/running/blocked, last-run `timestamp`, status timestamps and tags. No temporary-program, Group, Widget, snapshot, graph or authored-source expansion. |
| `/app/presentation/automation/overview/get` | `{automationId}`: nonblank, trimmed | `{id,name,type,enabled,runnable,corrupted,blocked,saveIncomplete?,widgets}`. Selected metadata, status, cloud quota display state and owned Widget configuration/current values. No source, graph traversal, running history or snapshots. Missing definition is not-found. |
| `/app/presentation/automation/placements/get` | `{automationId}`: nonblank, trimmed | `{canvases:[{id,name,containsAutomation}]}`. Matches both owned Widget-to-Card relations and dedicated Automation Cards by `meta.automationId`, against Canvas leaf IDs including composition members. Backends read current-Space Card records to resolve these business references; responses contain only Canvas labels and membership flags. Retains existing default-canvas initialization when the Space has none. |

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

List items, selected summaries and overview may include `pendingOperation`
(`create`, `update` or `delete`). Omission means the definition is ready for
ordinary management. An incomplete creation can only be deleted; an incomplete
update can be saved again or deleted; an incomplete deletion can be continued
by deleting the same ID. All three states block new execution. The
`saveIncomplete` field remains available to older clients and is true when a
pending operation is present.

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

## Selected summaries and configuration editing

`/app/presentation/automation/summaries/get` accepts the same bounded selected
`{ids}` as names. It returns `{items:[{requestedId,found,resource}]}` in normalized input
order. A found resource contains ListItem fields plus `automationId`, `temp`,
nullable `lastRunAt` and `expireAt`. Missing items omit resource. It supports
stored Standard, Temporary and generated programs; it does not load script
bodies, parse source, expand Widgets, snapshots or graphs. Public program status
also uses metadata rather than downloading authored source.

| Target | Input | Result / retained behavior |
| --- | --- | --- |
| `/app/presentation/automation/overview/init` | `{automationId?,inspireId?,inspireVersion?,snapshotId?,args?}` | Existing editor session, required installs, AutomationOverview and already-loaded metadata. Snapshot takes precedence over existing program, then Inspire. Uses the authenticated Client and existing staged-Inspire developer session. |
| `/app/presentation/automation/overview/config/provide` | `{sessionId,var,input}` | Existing option query and editor revision. |
| `/app/presentation/automation/overview/config/update` | `{sessionId,var,inputs?,widgetize?}` | Existing config evaluation, diagnostics and revision. |
| `/app/presentation/automation/overview/save` | `{sessionId,automationId?,inspireId?,inspireVersion?,snapshotId?,preferredName?,debug?,addCard?,canvasIds?}` | `{automationId,runnable,enabled}`. Existing single-save guard, Client ownership, prepare/save/complete revision checks and internal normalized-source persistence. Legacy snapshotId/addCard/canvasIds are accepted but do not perform placement; frontend creates placements separately. |
| `/app/presentation/automation/overview/cancel` | `{sessionId}` | Canonical acknowledgement without data. Existing Client ownership check and session cleanup. |
| `/app/presentation/automation/copy` | `{automationId,name?}` | `{automationId,runnable,enabled}`. Copies the existing script/template metadata using the internal save operation. |
| `/app/presentation/automation/enroll` | `{automationId,name?}` | `{automationId,runnable,enabled}`. Existing enrollment behavior. |
| `/app/presentation/automation/share` | `{automationId}` | Existing `{sharedId}` from the share service, using trusted caller identity. |
| `/app/presentation/automation/generated/enable` | `{automationId}` | Existing AutomationStatus for the selected generated program. Enables listening; does not force execution. |
| `/app/presentation/automation/generated/disable` | `{automationId}` | Existing AutomationStatus after disabling. |
| `/app/presentation/automation/generated/run` | `{automationId,timezone?,locationSymbol?}` | Canonical acknowledgement without data. Forces the existing stored generated program's threads, including Recurring programs. Runtime execution remains asynchronous. |

Generated control accepts only stored `P-`, `S-`, `G-` program IDs. It does not
rename generated programs, control Group entities, or widen public user-program
start/stop. Actor and Client come from trusted ingress, never arbitrary sideData.
Activity logging is best effort and cannot turn a successful control into a
failure. These UI targets are not automatically exposed to Agent/CLI/MCP.

The script-source editor and all legacy `/automation/*` Controller routes are
retired. Configuration editing retains the existing internal save behavior;
public `/app/automation/save` retains its independent validated-source contract
for Agent/CLI/MCP. Internal compiler/format capabilities remain available.
