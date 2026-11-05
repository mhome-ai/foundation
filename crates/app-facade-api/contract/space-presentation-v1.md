# Space presentation and focused business readers

All calls use the standard App Facade envelope. These presentation targets are
UI reads, not Agent tools. They execute in the cloud and require user identity.

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
- `/app/space/location/list` accepts `{}` and returns `{locations}`. Each Location
  has `spaceId`, `locationId`, `name`, `address`, `timezone`, `countryCode`,
  `latitude`, `longitude`, and `altitude`. It executes according to Space mode,
  reading only the authoritative backend's Locations; it does not read Space
  details, members or preferences and has no cloud fallback in local mode.
- `/app/space/delete/status` accepts `{operationId}` and returns `accepted`,
  `operationId`, `spaceId`, `state`, `message`, and `deleted`. It queries the
  existing durable operation and never starts or restarts deletion. Authorization
  uses the operation initiator and authenticated user, so polling survives removal
  of membership and Space data. The request retains the operation's Space ID.
- `/app/space/beta/update` accepts `{betaMode: boolean}` and returns
  `{betaMode, poll}`. It saves through the existing Space Beta service. A true
  `poll` means the Client must wait for the Hub to apply the selected mode.

Invitation accept/reject remain membership operations and never select a Client's
Space. The UI explicitly uses the connection focus protocol after joining when
its flow intends to enter the joined Space. Current-Space switches must not be
discarded by the previous Space's request guard.

Client handling of presentation replies must unwrap Full Facade responses before
updating cloud-authoritative membership/connection caches. Legacy bootstrap
protocols and new UI reads must preserve that trust boundary during cutover.
