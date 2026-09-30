# Host permissions contract

Host coordinates permissions for this computer and the services it manages. It
never performs a service's protected business operations on that service's behalf.
Client/Desktop application permissions remain outside this report. No permission
request depends on a Space or enables a plugin in a Space.

This is a direct development cutover. There is one report model and no legacy
serialization, implicit component selection or alternative compatibility route.
Update Foundation consumers together after publishing the matching packages.

## Declaration, executor and authorization subject

Each service keeps its source-root `permission.yml`, packages it, and reports its
own requirements. Host stores installed declarations so stopped services remain
visible. A declaration says which feature needs access, not who the OS charges.

`PermissionObservation.processId` identifies the actual executing process. It is
not a grant identity. A helper may share its responsible application's consent.
A Rust service must obtain a sidecar's observation over its private transport when
that sidecar executes the protected operation; it must not substitute its own
Foundation query. Check the real backend used by that executor, including access
errors. Standalone launches report their actual launch context, which can differ
from Host-managed launches. Reading status must not start an absent sidecar.

`PermissionSubject` is an authorization scope established for the specific
permission, launch path and packaging profile. Host may group by this subject and
the full `PermissionKey`. Equal states, equal signing teams, a common parent,
service-supplied labels or environment variables alone do not establish sharing.
There is no generic public macOS query in this contract that discovers another
process's TCC grant owner. Verify responsible-application attribution using the
real signed launch chain before enabling a shared subject for a permission.
If attribution is unconfirmed, `subject` is null and that component stays separate.
Different Automation `targetBundleId` values always remain distinct grants.

A grouped row preserves every `PermissionUse`, its observation and any collection
error. There is deliberately no group-level grant/state that can erase a denial,
an unknown executor or conflicting observations. A stopped service has no current
observation. Do not copy another service's grant into that empty observation.
Host chooses `requestComponentId` from available executors; it is null when none
can handle the request. `supportedActions` describes available mechanisms only.

## Transport and actions

- `GET /v1/permissions` returns `HostPermissions` for installed requirements,
  including stopped services and declaration/collection failures.
- `POST /internal/permissions/request` performs one explicit request or probe in
  the selected service's actual execution context, then refreshes the report.
- `POST /internal/permissions/open-settings` opens the platform settings pane.
  Opening Settings is not a grant; status must be refreshed afterwards.
- Both POSTs take `HostPermissionRequest`. `hostId`, `componentId` and `permission`
  are mandatory. The component must declare that exact key. Host derives grouping
  itself; the caller cannot assert a subject, locality or an arbitrary Settings URL.
- Require valid local control credentials and matching Host identity. Native
  Desktop and CLI use Client IPC. Remote Hosts accept status only. Do not send
  local control credentials to discovered addresses or redirects. A future Hub
  read can relay a report but cannot relay mutations or decide grants.
- A service's `GET /internal/permissions` returns `ServicePermissions`. The outer
  PID identifies the reporting service; individual observations can identify its
  sidecar. The service rejects requests outside its declared requirements.

Host coalesces in-flight requests for an established shared subject and key, or
for the individual executor and key when sharing is unknown. Transport timeout
must not start a duplicate native request. Refresh all affected service reports
when an operation completes. Bound collection concurrency and the whole snapshot
latency; a stalled executor must not delay every other service indefinitely.

## Consent and actual access

`PermissionObservation.state` describes application consent. `access` is a
separate backend observation. A successful consent check does not establish that
a device is powered on, present, correctly configured or usable. A successful
scan does not prove that every connection or operation will succeed.

- `system`: passive native authorization query and its timestamp.
- `platform`: platform consent semantics, not measured resource availability.
- `probe`: historical evidence from an explicit operation, with its timestamp.
- `unavailable`: a reliable consent observation could not be obtained.
- `notRequired`: this native backend has no application-consent step. It does not
  bypass users/groups, D-Bus policies, device rules, sessions, portals or sandboxes.
- `access`: the actual backend's checked condition, timestamp and result. Missing
  access evidence is unknown, not proof of success. Its detail describes only the
  operation/condition actually observed. Do not relabel a hardware failure as OS
  user denial or a successful consent query as a successful resource probe.

Passive report reads never request authorization, probe Local Network, scan
Bluetooth, launch an Automation target, restart a service or open Settings.
Local Network's last explicit probe is historical; ordinary Refresh cannot make
it a fresh OS setting. Keep cached reports visibly stale when collection fails.
Missing/malformed declarations remain errors; an empty list is not proof of a
complete inventory or successful access. Do not grant by default on other OSes.

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
native application-consent step separately from the backend's access results.
Host and services must not claim access merely because there is no macOS TCC.
An installation preflight collects concrete missing conditions. An administrator
may authorize a bounded, idempotent setup step for those conditions; service
manifests must not supply privileged commands. Runtime processes remain ordinary
users. Account/session refresh requirements remain explicit. New device or
capability requirements can require additional setup. Container and sandbox
policies are separate deployment conditions, not automatically inherited grants.

## Acceptance

Test passive reads without prompts; real signed allow/deny/revoke/relaunch
behavior; shared grants across two executors; independent/unknown subjects;
sidecar and stopped-service observations; conflicting per-service states;
per-target Automation; cross-host mutation rejection; required component
selection; incomplete manifests; bounded collection; coalesced requests; and
Linux policy denial versus missing/disabled resources. Test Linux setup first as
a dry-run and verify effective access in the intended ordinary runtime session.
Signing fixtures and mocked status tests do not prove TCC attribution.
