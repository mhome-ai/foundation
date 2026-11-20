# Host permissions contract

Host coordinates permissions for this computer and the services it manages. It
never performs a service's protected business operations on that service's behalf.
Client/Desktop application permissions remain outside this report. No permission
request depends on a Space or enables a plugin in a Space.

This is a direct development cutover. There is one report model and no legacy
serialization, implicit component selection or alternative compatibility route.
Update Foundation consumers together after publishing the matching packages.

## Declaration and executor

Each service keeps its source-root `permission.yml` and packages it. The installed
package is the sole source of desired permissions. Host loads declarations into
its installed-service inventory on startup and when packages change. Status reads
use that memory snapshot; they do not scan packages or maintain a database copy.
Stopped services retain their requirements. Invalid declarations remain visible.

Each `HostPermission` is one component and one permission: `componentId`,
`componentName`, `permission`, `enforcement`, `feature`, `reason`, `observation`, `error`, and
`supportedActions`. There are no grouped subjects or selected substitute executors.
`PermissionObservation.processId` identifies the actual executing process, not an
OS grant identity. A sidecar reports its own observation through its service.
Reading status must not start an absent sidecar. The service shell owns permission
policy and business activation. It may observe a permission shared with its Core
helper only when the platform launch/signing contract establishes the same
authorization identity. Do not relabel the shell observation with the helper PID.
An unrelated execution context requires its own observation; it does not own a
second permission policy state machine.

Requirements default to `enforcement: startup`: the service starts its management
control plane, checks consent, and keeps business initialization/admission closed
until all startup requirements are satisfied. Waiting for a user is not a process
failure and does not consume the business initialization timeout. Explicit
`enforcement: operation` requirements are checked by the affected operation; Mac
Apple Events use this policy because their target applications need not be running
at service startup. A permission grant is not business readiness. Native request
submission, authorization, and successful business initialization remain distinct.

A helper can share its responsible application's consent without any grouping
fields in this protocol. Equal PIDs, signing teams or states alone do not establish
shared grants. Automation keys remain distinct per target application.

## Transport and actions

- `GET /v1/permissions` returns `HostPermissions` for installed requirements,
  including stopped services and declaration/collection failures.
- `POST /internal/permissions/request` performs one explicit request or probe in
  the selected service's actual execution context. It returns `{ "ok": true }`
  once submitted, without collecting another snapshot or waiting for user consent.
- `POST /internal/permissions/open-settings` opens the platform settings pane.
  It returns `{ "ok": true }` after opening. Status is queried separately.
- Both POSTs take `HostPermissionRequest`. `hostId`, `componentId` and `permission`
  are mandatory. The component must declare that exact key. The caller cannot assert locality
  or an arbitrary Settings URL.
- Require valid local control credentials and matching Host identity. Native
  Desktop and CLI use Client IPC. Remote Hosts accept status only. Do not send
  local control credentials to discovered addresses or redirects. A future Hub
  read can relay a report but cannot relay mutations or decide grants.
- A service's `GET /internal/permissions` returns `ServicePermissions`. The outer
  PID identifies the reporting service; individual observations can identify its
  sidecar. The service rejects requests outside its declared requirements.

Foundation owns native request deduplication in the actual executor. Transport
timeouts do not release that ownership. UI polls status every two seconds while
managing permissions and refreshes on focus; a pending read is shared. CLI batch
requests query once after submission. Bound collection concurrency and the whole
snapshot latency so a stalled executor cannot hold every other service indefinitely.

## Authorization observations

`PermissionObservation.state` describes application consent only. The report
contains declared requirements and authorization observations; it does not collect
or carry resource availability or business-operation results. `granted` and
`notRequired` satisfy a requirement when the observation has valid evidence and
no errors. Missing, denied or unreadable observations remain unresolved.

- `system`: passive native authorization query and its timestamp.
- `platform`: platform consent semantics, not measured resource availability.
- `probe`: historical evidence from an explicit operation, with its timestamp.
- `unavailable`: a reliable consent observation could not be obtained.
- `notRequired`: this native backend has no application-consent step. It does not
  bypass users/groups, D-Bus policies, device rules, sessions, portals or sandboxes.

Passive report reads never request authorization, probe Local Network, scan
Bluetooth, launch an Automation target, restart a service or open Settings.
Local Network's last explicit probe is historical; ordinary Refresh cannot make
it a fresh OS setting. Keep cached reports visibly stale when collection fails.
Missing/malformed declarations remain errors; an empty list is not proof of a
complete inventory. Do not grant by default on other OSes.

## macOS and Linux

macOS keeps Host's existing responsible application and managed process tree.
Share identity only for validated permission/launch combinations. Services and
sidecars retain their own valid signatures and required hardened-runtime
entitlements. Signatures and launch association alone are not proof that an OS
grant is shared. Unsupported sharing stays a separate request; it never causes
business operations to migrate into Host. Never modify a signed plist in place.

Foundation's macOS status functions remain in-process and passive. Bluetooth's
explicit request initializes Core Bluetooth to request consent; merely opening
Settings is not a first-request implementation. When the actual user of BLE is
Matter's Node sidecar, obtain and request its state there. No Matter.js fork or
Host BLE broker is implied by this contract.

Linux native services use the configured ordinary runtime account and existing
system interfaces, including BlueZ D-Bus. Foundation reports the absence of a
native application-consent step as `notRequired`. Resource health belongs to the
service's own diagnostics and does not affect this permission report.
An installation preflight collects concrete missing conditions. An administrator
may authorize a bounded, idempotent setup step for those conditions; service
manifests must not supply privileged commands. Runtime processes remain ordinary
users. Account/session refresh requirements remain explicit. New device or
capability requirements can require additional setup. Container and sandbox
policies are separate deployment conditions, not automatically inherited grants.

## Acceptance

Test passive reads without prompts; real signed allow/deny/revoke/relaunch
behavior; shared grants across two executors; sidecar and stopped-service observations; conflicting per-component states;
per-target Automation; cross-host mutation rejection; required component
selection; incomplete manifests; bounded collection; native request deduplication; and
Linux `notRequired` without any resource observation. Test Linux setup first as
a dry-run and verify effective access in the intended ordinary runtime session.
Signing fixtures and mocked status tests do not prove TCC attribution.

## Service state and invalidation

The service shell publishes its process generation, PID, revision, lifecycle
phase and `ServicePermissionState` at `GET /internal/service/state` and
`POST /internal/service/state/wait`. The latter accepts an optional
`processGeneration`/`revision` cursor and waits up to 25 seconds for a newer state.
These management endpoints are loopback-only. A state is not proof of a grant;
individual permission observations remain the evidence. Waiting does not mean
business readiness, and an old process's state is never adopted by PID alone.
The private `/internal/service/status` report also places this exact state under
`details.serviceRuntime`. This namespace is reserved for shell lifecycle data;
business diagnostics may continue using `details.runtime`. Host treats permission
waiting, business initialization and business shutdown as `activating`, and only
accepts `ready` after the shell confirms both permission and business readiness.

An authenticated `POST /v1/changes/wait` on Host accepts the same cursor field
names and returns `{hostId,cursor}`. This is ordinary read authority, not permission
to open OS prompts. Consumers read an initial/current snapshot, subscribe from its
cursor, and reread after invalidation or reconnect. Cursors are in-memory latest
state, not a persistent event queue. Native callbacks notify changes after detection;
platforms without a full OS settings feed require bounded passive observation.
Host shutdown cancels idle waits. Local Client permission `snapshot` reads local
and already trusted, discovered remote Hosts independently of Core/Space. It never
requests consent; remote prompts and Settings remain prohibited.
