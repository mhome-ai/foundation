# mhome-os-permissions

In-process operating-system permission calls shared by MeowLink Client, MeowLink Host, and local
plugins.

The crate maps one permission key to a passive status read, an in-process request, or opening the
matching System Settings pane. It does not decide which process owns a key, store install records,
or serve HTTP. A call is attributed to the process that links this crate and invokes it.

macOS implements the calls. Other systems report the permission as unsupported and reject requests.
