# Resource read boundaries

Business reads separate persisted definitions/configuration from current observations,
chronological history, and one-hop resource relationships. Tenant and Space are taken
from authenticated request context. All targets require Space read authority and
follow the Space execution authority on both local Core and cloud.

## Shared business reads

| Target | Input | Business data |
| --- | --- | --- |
| `/app/device/detail/get` | `{ids}` | Existing ordered `items` with `device` definitions, capabilities, group membership, physical Twin enablement or Group application state, source metadata; no online or state fields |
| `/app/device/state/get` | `{ids}` | Ordered `items` with `resource: {entityId, online?, onlineUpdatedAtSec?, latest}` |
| `/app/widget/detail/get` | `{ids}` | Existing ordered `items` with `widget` configuration; no value, next or nextTimestamp |
| `/app/widget/value/get` | `{ids}` | Ordered `items` with `resource: {widgetId, revision, value, next?, nextTimestamp?}` |
| `/app/automation/status` | `{ids}` | Ordered `items` with `resource: {automationId, enabled, runnable, running, blocked, lastRunAt}` |
| `/app/graph/relations/get` | `{resource: {kind, id}}` | `{resource, relations: [{from, relation, to, capabilityIds?}]}` |

Device list and detail discriminate status using `group`: physical Devices (`false`)
always include boolean `active` and omit `applyState`; Groups (`true`) always include
`applyState` (`applying`, `ready`, `failed`, `deleting`) and omit `active`. Both Full
and Essential preserve these fields. `active` describes physical Twin enablement,
not on/off or reachability. Group state comes from its stored definition, never a
Twin lookup; `ready` confirms application, not member reachability. Group diagnostics
and application errors are available through Group detail. Device enable/disable
accepts physical IDs only. This does not add Group enablement or alter lifecycle writes.

Observation batches accept 1–100 IDs, reject blanks, trim and deduplicate preserving
request order. Every normalized ID gets `{requestedId, found, resource?}`; missing
resources have `found: false, reason: "not_found"`. A known Device with no observations
has `found: true` and `latest: []`. Unknown reachability omits `online`; it is not false.
Device state supports visible physical devices and Device groups. Automation status
accepts user program IDs, including retained one-time programs, not groups or internal
Twin programs. Widget value supports Data, Button and Circadian resources. JSON null
is a value, not a missing resource. Button values may be null.

`latest` contains one record per capability: `capabilityId`, optional `state`, `event`,
`log`. These preserve the decoded Twin observation objects and their second timestamps.
They contain no persistence tenant/Space keys. Reads use stored latest snapshots and
retain the existing history fallback on cache miss; they do not refresh providers.
Definition reads do not load observations, history, related programs or visible Cards.

Graph kinds are `device`, `entity`, `automation`, `widget`, `card`. Directional relations
are Automation `uses` Device/Entity (with capability IDs), Automation `owns` Widget,
visible Card `presents` a resource, and generated Automation `implements` Device.
Generated program references are not editable user programs. Existing Managers and
indexes own these relationships; this read introduces no graph database or traversal.
It returns IDs, not recursively expanded definitions, scripts, names or runtime data.
Hidden Device render anchors are not public Card references.

Full uses the standard `{status, data}` result with structured business data. Essential
uses the same envelope with YAML `data`; timestamps are presented in the Home timezone.
Device list additionally omits connection/manufacturer metadata in Essential. New
observation and graph reads preserve their bounded business fields in Essential.

## UI-only reads

- `/app/device/view/get`, `{deviceId}`: `{deviceId, primaryViewId, views, widgets}`.
  Reads the unique hidden Device render definition, preserving exact Widget/Twin
  bindings. Disabled devices can have empty definitions. Multiple anchors fail rather
  than choosing an arbitrary visible Card. Widgets contain configuration, never values.
- `/app/device/history/series/get`, `{deviceId, capabilityIds?: []}`:
  `{deviceId, series}` with per-capability decoded `states`, `events`, `logs`. Empty
  capability selection reads all series. This preserves chart structure; the shared
  Device history Facade remains a bounded, merged business timeline.
- `/app/dashboard/canvas/content/get`, `{canvasId}`: Canvas metadata plus Cards' static
  titles, view bindings, layouts, settings, metadata and selected Widget definitions.
  No observations, history, related automations or suggestions are loaded.

The frontend batches selected observations after reading definitions. Automation Cards
also read selected Automation definitions for their name and type. Canvas IDs are
split into bounded batches and deduplicated across Cards. History is requested by an
open detail renderer; relationships by the view that displays them. Pushes received
during a snapshot request are applied after it lands. Switching scopes or disposing
controllers prevents late responses from landing in another Space.

House continues to use `/app/entity/presentation/get` and legacy Twin reads as its
separate development boundary. Pod Deck retains its existing complete Card content
reader. These consumers are not silently migrated to static Canvas responses.

Generated Twin program inspection retains its internal `/automation/get`,
`/automation/status` and `/automation/info` UI readers. These IDs are not accepted
by the shared user-program Automation detail/status contract. User program detail
pages use the shared definition, status and graph readers.
