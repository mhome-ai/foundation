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
UI membership refresh uses `/app/presentation/space/list`. CLI `space list` forwards
`/app/space/list` and its canonical `{spaces}` DTO with `spaceId`, `name`, `mode`,
`llmMode` and `lifecycleState`. It never expands per-Space context or adds local
active-Space state and Hub routes. Read selected details, members and preferences
through their own Facades. CLI selection uses `/app/client/space/focus`; it returns
`{focusedSpaceId}` and leaves membership validation/persistence to the receiving
Client and authoritative executor. `/focus` has no compatibility alias.

Space commands pass the caller's Full/Essential choice to the Facade. Prepare
previews remain fixed control objects. Whole-Space deletion drives the existing
asynchronous protocol using structured Full control reads, then returns the
unchanged `/app/space/delete/status` DTO in the requested detail. It does not
rewrite `action` or hide operation fields. A successful status read may describe
`DELETE_FAILED`; waiting for that failed operation still exits unsuccessfully.

## Space mutation outcomes

The backend settles member invite/remove outcomes before presentation. A request
with no failed items succeeds, including already-pending/already-member invitations
and removal of absent memberships. Fulfilled and failed items together return
`partial`; failure of every requested item returns `error` with the complete
per-item result in diagnostics. Full and Essential retain the same outcome.
Location writes with `NotCreated`, `NotUpdated` or `NotDeleted` return `error`,
with the original write result in diagnostics. Prepared previews describe planned
work and never prove effects. Reading a failed deletion operation remains a
successful read; accepting deletion never means erasure has completed.

`/app/space/member/update` accepts `{removeEmails?: string[], inviteEmails?: string[]}`
with at least one requested change. UI member editing and the human-operated CLI
share this business composite. The backend normalizes/resolves all inputs and
rejects an OWNER removal before any write. It then removes memberships and sends
invitations under one Space mutation lock; it does not promise rollback. The same
email cannot be removed and invited together. Its complete result is
`{action, spaceId, removal, invitation, problems}`; removal/invitation preserve
those individual operations' DTOs. If invitation execution becomes unconfirmed,
`invitation` is null and `problems` explains that membership must be inspected
before retrying. The backend settles the overall outcome. It supports direct and
prepared modes; previews perform no writes. This operation does not extend the
native Agent, MCP or external Skill boundary for member removal.
