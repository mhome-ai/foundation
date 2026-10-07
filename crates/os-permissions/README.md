# mhome-os-permissions

In-process operating-system permission calls shared by MeowLink Client, MeowLink Host, and local
plugins.

The crate maps one permission key to a passive status read, an explicit in-process request, or
opening the matching System Settings pane. Its observation records the executing process's PID,
not the OS grant owner. macOS may attribute a helper's access to its responsible application.
The crate does not discover that identity, inspect another process, persist grants, or serve HTTP.

macOS implements native authorization queries. `request()` starts authorization and returns
without waiting for the user's decision. Poll `status()` to update a permission page; accepting a
request is not permission approval. The process retains at most one in-flight request per key
(including the Automation target). Losing an HTTP caller does not cancel or duplicate it. Native
completion callbacks release request objects; only the synchronous Apple Events API needs a worker.
No UI operation identifier, persistent grant record, or client callback subscription is required.

Only explicit Bluetooth requests initialize Core Bluetooth. Only an explicit Local Network request
starts a process-owned Bonjour monitor. Its state callback updates the latest network observation,
including a later waiting/error state. Repeated explicit requests replace that monitor, with at most
one live browser. Status reads do not start a probe or raise a prompt. Local Network has no general
passive authorization API: its report is operation evidence, with the original observation time,
not a guaranteed live reading of the System Settings switch. A fresh process has no such evidence.
The monitor does not enumerate or return discovered devices.

Opening System Settings is a separate action. Returning from Settings does not establish a grant;
the caller should resume passive polling. When no new network observation exists, preserve its
observation time rather than presenting the last successful probe as a fresh authorization check.

Linux reports `notRequired` / `platform` for native Bluetooth, local networking and microphone
consent. Bluetooth stays `notRequired` whatever the process groups are, because BlueZ policy,
not group membership, decides access. After BlueZ refuses a request (`AccessDenied` /
`NotAuthorized`), `bluetooth_access_denied_hint()` explains a missing `bluetooth` group: how to add
the user, and that a systemd user service only gains the group after the user service manager
restarts (`sudo systemctl restart user@<uid>.service`) or a reboot.

Windows desktop apps need no Bluetooth consent: Bluetooth reports `notRequired` / `platform` when
a radio is present and `unsupported` when there is none. Permission reports contain authorization only; device/session availability and
business-operation results belong to service diagnostics. A sandbox or portal can impose
additional restrictions. macOS-specific Reminders and Apple Events report `unsupported` on Linux.
Other permissions and operating systems explicitly report an unimplemented adapter.

For a service with a sidecar, obtain observations in the actual resource executor and relay them
through the service. Checking the Rust service alone does not establish the sidecar's access.
Standalone launches describe that launch context, which may differ from a Host-managed launch.
