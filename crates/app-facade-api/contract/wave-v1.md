# Wave authorization management

These Hub-routed App Facades require current-Space Manage access and an active
local Hub. They do not make Wave devices Hosts, Nodes or Plugins. No device URL,
Space/Hub selector, grant, certificate or private key is an input/output.

- `/app/wave/discover`: `{transport:"lan"|"ble"}` ->
  `{candidates:[{candidateId,deviceId,proof,purpose}]}`.
  LAN discovery browses `_meow-wave._tcp.local.`; it never invokes BLE Relay.
  BLE discovery searches only bound relays for pure BLE operation. It does not
  discover unconfigured Wi-Fi devices for network setup. The caller selects
  exactly one transport; there is no implicit discovery or fallback.
  Results are untrusted location hints, scoped to this caller, Space and Hub
  generation and transport, expiring after two minutes. They never grant operation
  authority.
- `/app/wave/pair` is retired. Core rejects it with guidance to select the
  Integration and use `/app/integration/connect`. Its constant/input DTO remain
  readable for older clients; this route cannot enroll a device.
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

Standard onboarding uses `/app/integration/connect` for a concrete local Integration
whose compiled model declares `connectAdapter: wave`. It uses the same interactive
start/input/resolve/next/confirm/read/retry lifecycle as other custom connections.
The built-in typed form supports PIN6 and explicitly opened initial no-PIN windows.
Core checks the selected resource before pairing, verifies it against authenticated
inventory after grant confirmation, writes the normal IntegrationInstall, and
publishes compatible entities. It creates no ordinary Connection record.

The lifecycle response includes `wave: {deviceId,grantState,inventoryState,installed,
entityIds,warnings,reason?}`. Completed projection is `status: done`; interrupted
setup is `status: failed, recoverable: true, canCancel: false`, preserving committed
facts. Retry uses the durable accepted identity and source, never a new pairing.
Additional unavailable capabilities warn without erasing compatible entities.
Preview selection does not install a business Integration. Integration uninstall
and physical grant removal are separate operations.

Credentials remain Core-local, outside ordinary Connection/privateData/Twin/sync.
Explicit Hub migration is outside this increment. Physical target acceptance and publication remain separate from source implementation. Commands are never replayed.
