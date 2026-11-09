# Device UI presentation

Device-specific UI adapters use `/app/presentation/device/*`. All three are
read-only, use the standard App Facade envelope and follow Space-mode execution:
cloud Spaces execute in the cloud; local Spaces execute on their selected Hub.
They have no native Agent, external CLI Skill, MCP or CLI entrypoint. Shared
`/app/device/state/get`, list, detail and bounded history remain domain APIs.

- `/app/presentation/device/view/get`, `{deviceId}`: `{deviceId, primaryViewId, views, widgets, twin}`.
  Reads the unique hidden Device render definition, preserving exact Widget/Twin
  bindings. Disabled devices can have empty definitions. Multiple anchors fail rather
  than choosing an arbitrary visible Card. Widgets contain configuration, never values. The optional `twin` metadata is
  returned even when a known device has no render anchor.
- `/app/presentation/device/history/series/get`, `{deviceId, capabilityIds?: []}`:
  `{deviceId, series}` with per-capability decoded `states`, `events`, `logs`. Empty
  capability selection reads all series. This preserves chart structure; the shared
  Device history Facade remains a bounded, merged business timeline.
- `/app/presentation/device/online/list`, `{entityIds?: []}`:
  `{devices:[{entityId,online,updatedAtSec}]}`. Normalizes and deduplicates selected
  IDs, preserves request order and returns only IDs with stored online records.
  An empty selection returns no rows. This preserves the existing UI projection:
  `online` is true for the stored `online` observation and false for other stored
  states; an absent record is omitted. It never probes a device or reads capability
  observations. The shared state API instead omits unknown `online` fields.

Device details and Device Group State tabs read static bindings first, then use
shared selected `/app/device/state/get` and `/app/widget/value/get` observations. Canvas
content reuses static Card assembly; it does not call the single-device view API.
History renderers request chart series only when needed. The Devices directory
reads selected online rows after loading its catalog. These reads never refresh
providers, change Twin lifecycle or initialize new business resources.

The backend adapters own visibility, unique render-anchor validation and existing
series filtering. Managers, persistence tables, group aggregation, generated
Automations and push protocols are unchanged. Only the namespaced UI routes are registered.
