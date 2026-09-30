# Host permissions contract

This contract is prepared for the Host permission implementation. Adding these
DTOs does not implement an OS adapter, HTTP handler, CLI command or UI.

## Ownership and transport

The subject is `ai.mhome.meowlink.hostd` / MeowLink Host. Client permissions belong
to `ai.mhome.meowlink.clientd` and are not returned here. Never use a Space ID to
identify a permission grant. A permission does not enable a plugin in a Space.

- `GET /v1/permissions`: passive, per installed-package requirements, including
  stopped Core/Nodes. The same PermissionKey on another process stays a separate
  row. Do not merge those rows: the operating system grants the process that
  calls the API.
- `POST /internal/permissions/request`: explicit local request or probe, executed
  in the process that owns that permission.
- `POST /internal/permissions/open-settings`: explicit local navigation. Opening
  Settings is not a grant; the caller must read status again.
- Both POSTs take HostPermissionRequest and return a refreshed HostPermissions.
  `componentId` selects the declaring installed service, or `host` for the Host
  itself. The selected component must declare the exact permission key. For
  compatibility, an omitted component is accepted only when exactly one
  component declares that key; ambiguous requests must fail without prompting.
  They must reject the wrong hostId, nonlocal control routes and missing/invalid
  local control credentials. Host must require the control token independently
  of the Client's local identity check. Neither hostId nor isLocal authenticates
  a request. Never send that credential to discovered addresses or redirects.
  Mutations stay on the machine's local control socket.
- Native Desktop and CLI use Client IPC for this computer. A remote Host accepts
  status only. A Hub may relay `GET /v1/permissions` for a connected local Space;
  Lion does not store or decide the grant. Mutations are not a facade operation
  and never require a Space.


`PermissionKey` includes targetBundleId for Automation; Music, Calendar and
Messages are independent grants. Unknown future declaration keys must appear as
component declaration errors until supported; they must not vanish or be counted
as granted. Requirement reasons describe affected features, not blanket Host
startup prerequisites. Linux currently has no macOS TCC permissions; return an
explicit Linux snapshot rather than synthesize macOS grants.

## Observation semantics

Reading status never requests authorization, probes Local Network, launches a
target app, restarts a service, or opens Settings. Permissions are checked on
opening the permissions view and explicit Refresh, not in the five-second
hardware metrics poll. A remote view has no actionable permission buttons.

- `system`: passive OS query and its timestamp.
- `probe`: the last explicit Local Network probe and its timestamp; historical
  evidence must be labelled as such and never presented as a current OS setting.
- `unavailable`: status cannot be obtained; unknown/unsupported is not denied.
- Bluetooth hardware off, an absent target app and transport failure are not OS
  denial. Expose the applicable unknown/error instead of fabricating a grant.
- Invalid or unreadable installed manifests yield declarationErrors. Empty
  permissions plus declaration errors must not show an "all allowed" state.
- In-flight permission requests are coalesced per key. Waiting is bounded, the
  request remains observable, and no detached retry loop prompts repeatedly.
- A failed refresh retains a previous snapshot only as stale; the native Client/UI must preserve
  this distinction instead of presenting cached grants as current.

## Packaging and rollout prerequisites

Read and request each permission in the process that calls the API. Host reads
local network and Bluetooth. AudioBridge reads the microphone. The Mac service
reads Reminders and per-app automation. The desktop shell's camera, microphone
and notification grants are separate and are not part of this snapshot. Do not
add a permission-helper identity, and do not have the desktop process request a
grant that belongs to Host or a service. Check the signed calling bundle's usage
descriptions. Opening System Settings may be performed by Host. The Host carries
the supported declaration vocabulary. Service installation must compare a
package's requirements with those capabilities before activation and require a
Host update when missing. Never patch a signed plist in place.

Initial declarations follow actual implementations: Host local discovery,
Matter Bluetooth commissioning, AudioBridge microphone recording, Mac Reminders
and per-app Automation. Camera is a network camera service; its name alone does
not justify requesting macOS camera permission.

Publish the canonical core-api contracts and matching npm protocol package
before updating consumer pins/lockfiles and implementing Host/Client/UI/CLI
routes. Regenerate native and Pallas protocol projections from those contracts.
A Hub facade read, when added, only relays Host `GET /v1/permissions`. It is not
implemented by this contract revision. An endpoint must not be considered
implemented merely because it appears in a routing manifest.

Acceptance requires passive-query/no-prompt tests, cross-host mutation rejection,
no-Space native access, stopped-service declarations, incomplete manifests,
per-target Automation, and actual allow/deny/relaunch checks in the process
that calls the API. Signing fixtures do not prove TCC authorization attribution.
