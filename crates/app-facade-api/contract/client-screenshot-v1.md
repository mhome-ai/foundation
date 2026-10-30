# Native app screenshot support

`/app/client/screenshot/report` accepts `{ "supported": boolean }` and returns
an empty business object in the standard Facade response. It reports only the
calling authenticated app connection, in its current Space; caller identity is
never supplied in the input. Agent, CLI and service callers cannot report for a
client. Authorization is Space read plus an authenticated, live client binding.

Support is ephemeral connection metadata, reset on reconnect. A positive report
receives a server-derived 90-second expiry. A visible, ready UI renews every
30 seconds and withdraws on hide or detach; ordinary socket heartbeats do not
renew this support lease. An expired report is unavailable even if its daemon
remains connected. Native clients
report true only after their installed WebView bridge confirms viewport capture
support. Web/PWA, messaging and headless clients never report true. Reporting
support grants no permission to capture; `basic.app.command.getScreenshot` uses
normal capability authorization and must independently resolve one live client.

The command captures the visible MeowLink WebView viewport, including app-owned
web overlays. It requires the foreground app to display the requested Space.
It must recheck the session and Space revision around capture and storage, and
must not bring the app to the foreground or return an older image on failure.
Native platform overlays, OS UI, other windows and offscreen page content are
outside the capture. Hardware media composition depends on the platform WebView.

The client stores an image through `/app/artifact/put` (at most 1 MiB decoded),
and returns `{ screenshot: { uri }, capturedAt: { _type: "Timestamp", value } }`.
Timestamp value is Unix epoch milliseconds, matching the runtime value codec.
The command's transport budget is 15 seconds; other App commands retain theirs.
