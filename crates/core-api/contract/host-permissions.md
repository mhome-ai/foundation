# Host permissions contract

This contract is prepared for the Host permission implementation. Adding these
DTOs does not implement an OS adapter, HTTP handler, CLI command or UI.

## Ownership and transport

The subject is `ai.mhome.meowlink.hostd` / MeowLink Host. Client permissions belong
to `ai.mhome.meowlink.clientd` and are not returned here. Never use a Space ID to
identify a permission grant. A permission does not enable a plugin in a Space.

- `GET /v1/permissions`: passive, per installed-package requirements, including
  stopped Core/Nodes. Merge duplicate PermissionKey values, retain every use.
- `POST /internal/permissions/request`: explicit local macOS request/probe.
- `POST /internal/permissions/open-settings`: explicit local macOS navigation.
- Both POSTs take HostPermissionRequest and return a refreshed HostPermissions.
  They must reject the wrong hostId, nonlocal control routes and missing/invalid
  local control credentials. Host must require the control token independently
  of the Client's local identity check. Neither hostId nor isLocal authenticates
  a request. Never send that credential to discovered addresses or redirects.
- `/app/system/host/permissions/get` projects only the passive GET through a Hub.
  It has no OS mutation variants. The selected Space supplies the remote route,
  not the permission owner. Native Desktop and CLI use the Client local route
  without needing any Space or Hub.

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
- A failed refresh retains a previous snapshot only as stale; public Facade uses
  Observation<T> for this distinction.

## Packaging and rollout prerequisites

OS adapters must run as the Host authorization identity; no Desktop-side proxy
request and no new permission-helper identity. Check the actual signed Host
bundle's usage descriptions and hardened-runtime entitlements. The Host carries
the supported declaration vocabulary/capabilities. Service installation must
compare a package's requirements with those capabilities before activation and
require a Host update when missing. Never patch a signed plist in place.

Initial declarations follow actual implementations: Host local discovery,
Matter Bluetooth commissioning, AudioBridge microphone recording, Mac Reminders
and per-app Automation. Camera is a network camera service; its name alone does
not justify requesting macOS camera permission.

Release core-api first, then app-facade-api and its npm protocol package. Only
then update consumer pins/lockfiles, implement Host/Client/Core/UI/CLI routes and
regenerate Pallas protocol projections. An endpoint must not be considered
implemented merely because it appears in the routing manifest.

Acceptance requires passive-query/no-prompt tests, cross-host mutation rejection,
no-Space native access, stopped-service declarations, incomplete manifests,
per-target Automation, and actual allow/deny/relaunch checks under the signed
Host identity. Signing fixtures do not prove TCC authorization attribution.
