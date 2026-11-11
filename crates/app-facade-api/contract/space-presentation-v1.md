# Space presentation and focused business readers

All calls use the standard App Facade envelope. These presentation targets are
UI reads, not Agent tools. List/get execute in the cloud and require user identity;
the Location reader follows Space mode.

- `/app/presentation/space/list` accepts `{}` and returns `{spaces}`. Each item
  has `spaceId`, `name`, `mode`, `llmMode`, `betaMode`, `defaultActive`,
  `lifecycleState`, `connectionMode`, and `currentUserRole`. Nullable metadata
  remains nullable. It reads enrolled memberships and Space metadata, then live
  connection state; it never expands members, Locations, preferences or profiles.
- `/app/presentation/space/get` accepts `{includeMembers?: boolean}` (default
  false) and returns `{space, members?}`. `space` has the same summary shape.
  Only `includeMembers:true` reads the Space membership list and member profiles.
  Each member has `userId`, `email`, `name`, `pending` and `role`. Active members
  may read this presentation; management mutations still require their own grants.
- `/app/presentation/space/location/list` accepts `{}` and returns `{locations}`. Each Location
  has `spaceId`, `locationId`, `name`, `address`, `timezone`, `countryCode`,
  `latitude`, `longitude`, and `altitude`. It executes according to Space mode,
  reading only the authoritative backend's Locations; it does not read Space
  details, members or preferences and has no cloud fallback in local mode.

## Shared Space management

- `/app/space/delete/status` accepts `{operationId}` and returns `accepted`,
  `operationId`, `spaceId`, `state`, `message`, and `deleted`. It queries the
  existing durable operation and never starts or restarts deletion. Authorization
  uses the operation initiator and authenticated user, so polling survives removal
  of membership and Space data. The request retains the operation's Space ID.
- `/app/space/beta/update` accepts `{betaMode: boolean}` and returns
  `{betaMode, poll}`. It saves through the existing Space Beta service. A true
  `poll` means the Hub has not yet been verified to apply the selected mode.
  This owner-only business mutation is shared by UI, native Agent/Skill, MCP
  and CLI. A local Space may restart and interrupt running operations. The
  receipt confirms the saved selection; it never proves Hub restart completion.

Invitation accept/reject remain membership operations and never select a Client's
Space. The UI explicitly uses the connection focus protocol after joining when
its flow intends to enter the joined Space. Current-Space switches must not be
discarded by the previous Space's request guard.

The retired `/app/space/location/list` has no compatibility alias. The UI uses
`/app/presentation/space/location/list`; no independent Agent/MCP/CLI reader is exposed.

Client handling of presentation replies unwraps successful Full Facade responses
before updating cloud-authoritative membership/connection caches. Presentation
wire DTOs (`spaceId`, `llmMode`) are explicitly projected into native state;
login and Hub authorization retain their independently owned wire DTOs.

Legacy `/scope/list`, `/scope/get`, `/scope/preference/get`, `/scope/agent/get`,
and `/scope/location/list` readers are retired, without fallback aliases. Native
UI membership refresh uses `/app/presentation/space/list`; the CLI uses
`/app/space/list` for identity-only listing and selection checks, and presentation
listing for its full management view. Neither lightweight flow expands per-Space
context. `/focus` remains connection control, not a business Facade.
