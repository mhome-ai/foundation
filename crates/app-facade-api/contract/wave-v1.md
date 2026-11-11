# Native Wave LAN admission

These Hub-routed App Facades require current-Space Manage access and an active
local Hub. They do not make Wave devices Hosts, Nodes or Plugins. No device URL,
Space/Hub selector, grant, certificate or private key is an input/output.

- `/app/wave/discover`: `{}` -> `{candidates:[{candidateId,deviceId,proof,purpose}]}`.
  Core browses `_meow-wave._tcp.local.` and identifies bounded LAN candidates.
  Results are untrusted location hints, scoped to this caller, Space and Hub
  generation, expiring after two minutes. They never grant operation authority.
- `/app/wave/pair`: `{candidateId,pin?}` -> AdmissionResult below. This is a direct
  authenticated App interaction; Agent/MCP/cloud delegation cannot submit PINs.
  PIN6 is required when the selected device requires it. Omitting it is allowed
  only for an explicitly opened initial local-claim window. No fallback occurs.
  Retry reconciles an existing candidate; an uncertain result never rotates a key.
- `/app/wave/list`: `{}` -> `{bindings:[{deviceId,grantState}]}`. States are
  `candidate` or `committed`; this is authorization bookkeeping, not Device/Twin
  inventory, online state or evidence that a candidate committed on the device.
- `/app/wave/refresh`: `{deviceId}` -> AdmissionResult. Reconcile the same saved
  credential, validate full Entity self-description against installed contracts,
  then use ordinary Twin source synchronization. It cannot open a pairing window.

AdmissionResult is `{deviceId,grantState,inventoryState,entityIds,reason?}`.
`inventoryState:ready` means existing inventory synchronization completed;
`unavailable` with a committed grant is a partial result. Authorization may have
committed even when metadata/contract acquisition or inventory synchronization
fails. A later refresh uses the existing grant, without consuming another slot.
Missing/uninstalled or incompatible contracts never install code automatically.
Normal Device/capability APIs own subsequent observations, control and subscriptions.

Credentials remain Core-local, outside ordinary Connection/privateData/Twin/sync.
Explicit Hub migration is outside this increment. Removal/invitation UI, BLE and
physical target acceptance remain separate increments. Commands are never replayed.
