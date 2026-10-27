# Pod directory

`/app/pod/list` is a Space-mode-routed, direct, read-only Facade call with empty
input. It requires a personal User and read access to the current Space. It combines
all completed personal Pod registrations owned by that User with registered
Space-owned Pods in that Space. Another User's personal Pods and other Spaces'
Pods are excluded. Offline registrations remain; incomplete commissioning attempts
and nearby discovery candidates are not registrations.

A local Space composes this result in Core, reading its own Space inventory and
obtaining personal registrations through the authenticated private cloud control
bridge. A cloud Space composes it in cloud. LLM placement does not change this
owner. Cloud entrypoints forward local-Space business to Core; only an unavailable
Hub transport permits the explicit read-only registration fallback below.

The result contains `pods`, `spaceId`, `observedAtMs`, and `sourceErrors`.
Every row contains `registrationId`, `clientMode` (`user` or `space`), `registered`
(true), and `connection` with `spaceId`, `state` (`connected`, `disconnected`, or
`unknown`), and `source` (`hub`, `cloud`, or `unavailable`). Space rows also contain
`spaceId` and `appClientId`. Personal rows contain the saved `name`, `deviceId`,
`model`, `platform`, `firmwareVersion`, `issuedAt`, and `activatedAt`.
No tokens, enrollment grants, credential IDs or private keys are returned.

Connection is an observation in the indicated Space, not global power state.
Personal registration IDs are keyed by User and device public-key fingerprint;
personal connection IDs do not carry that fingerprint. The directory does not
join them by name or hardware ID. Personal connection state is therefore unknown.
Space registration IDs are their authenticated appClientId and can be correlated.
When a local Hub cannot be queried, the cloud's persisted Space registrations
remain visible with unknown connection state and `sourceErrors.space`. The cache
may omit registrations not yet synchronized; it must not imply a complete live
snapshot. Cloud presence lookup failures likewise retain saved Space records
as unknown, with an explicit source error. A successful empty presence lookup
can establish disconnection. Registration read failures fail the call instead
of fabricating an empty inventory; authorization and request failures remain errors.

`/app/pod/space/list` is an internal composition target, not a separate Agent,
MCP or CLI operation. Scope-mode routing reads the Hub for a local Space or cloud
inventory for a cloud Space. It returns only Space-owned Pod registrations.
UI, Agent, MCP and CLI all use `/app/pod/list`; UI groups by clientMode.

`/app/pod/rename` is a direct, idempotent write with exactly `registrationId` and
`name` (trimmed, 1–64 characters). The authenticated User is the owner; request
input cannot select a User. It updates only that personal registration's display
name and returns `{registrationId,name}`. It works while the Pod is offline and
does not change credentials, ownership, networking or the Pod's local settings.
It has no prepare/commit or HITL requirement. Space Pod rename is not provided.
The CLI is `meow pod rename <registrationId> <name>`; Agent/MCP use `pod_rename`.

Personal login approve/redeem and Space commissioning retain their existing
protocols. Factory reset is available only on the Pod itself, not remotely.
