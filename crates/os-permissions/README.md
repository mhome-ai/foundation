# mhome-os-permissions

In-process operating-system permission calls shared by MeowLink Client, MeowLink Host, and local
plugins.

The crate maps one permission key to a passive status read, an explicit in-process request, or
opening the matching System Settings pane. Its observation records the executing process's PID,
not the OS grant owner. macOS may attribute a helper's access to its responsible application.
The crate does not discover that identity, inspect another process, persist grants, or serve HTTP.

macOS implements the native authorization queries. Only an explicit Bluetooth request creates a
Core Bluetooth manager; status reads do not initialize Bluetooth. Call blocking requests on a
worker thread. Asynchronous native waits and the Local Network probe use `REQUEST_TIMEOUT_MS`;
an enclosing IPC timeout must allow additional time to refresh observations. The synchronous
Apple Events request is controlled by macOS and cannot be cancelled by this timeout. A transport
timeout must not start another native request while the previous one remains in flight.

Linux reports `notRequired` / `platform` for native Bluetooth, local networking and microphone
consent. This is **not** a successful resource-access check. The real backend must separately
report device/session availability and access-policy failures. A sandbox or portal can impose
additional restrictions. macOS-specific Reminders and Apple Events report `unsupported` on Linux.
Other operating systems explicitly report an unimplemented adapter.

For a service with a sidecar, obtain observations in the actual resource executor and relay them
through the service. Checking the Rust service alone does not establish the sidecar's access.
Standalone launches describe that launch context, which may differ from a Host-managed launch.
