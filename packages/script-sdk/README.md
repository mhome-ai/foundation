# Script authoring SDK

SDK 0.2.0 uses `@mhome/script` and the native `@mhome/script/host` module.
Rebuild and republish scripts authored with the former `@meowlink` package;
the old host-module name and SDK 0.1.0 are not compatibility entrypoints.

TypeScript authoring helpers for the JavaScript script migration. `defineScript`
declares Automation's named input/output fields once and infers the function's
TypeScript types. `t.object` and `t.value` preserve dynamic data; `t.list` preserves
element types and nesting. Integration entries use ordinary default-exported
functions and their role schemas instead of Automation contracts.

Functions return business data or throw `ScriptError`. This package does not
implement privileged IO itself. `getRuntimeContext`, `sleep`, `randomU64` and
`readArtifact` call the native host module; that import remains external in built
ESM. Calling these helpers in ordinary Node author tooling fails explicitly.
The source workspace is private; Foundation's script-package stager creates the
standalone `@mhome/script` npm package with built JavaScript and declarations.
It has no install hooks or sibling-source dependency. Actual registry publication
is a separate release step.

Network helpers include `httpRequest`, `httpReadBatch`, `dnsLookup`, `resolveMdns`,
`probeTcp` and `probeRtsp`. HTTP error status remains a response. Device probes
return an explicit success/failure outcome with diagnostic data; invocation
timeout/cancellation still rejects. Read batches preserve input order and use the
existing Space/connection cache. `requestNode` resolves the Node bound to the
current Integration connection; it cannot select an arbitrary client or Space.
`cryptoRequest` and `renderQrCode` use bounded native computation. Every helper
returns a Promise that must be awaited; internal timeouts cannot extend the caller
deadline. Core keeps native CPU capacity occupied until the work actually exits
even if the caller has timed out. No detached-task API is exposed.

`requestChannel`, `sendChannel`, `isChannelReady` and `disconnectChannel` use
Core-owned channels scoped to the current Integration connection. Use the channel
IDs from runtime connection data; entity channels additionally require `entityId`.
Requests preserve native values, including BigInt; sends use the channel's JSON
transport. `getChannelContext` reads the trusted context inside a channel hook.
Only an explicit disconnect stops the business channel; a script timeout does not.

`mapEndpoint`, `getEndpoint` and `unmapEndpoint` manage subscription routing in the
current Space. Store the opaque endpoint supplied by Core unchanged; the mapping
is not general-purpose provider storage. Core validates route syntax and Space
ownership. `webhookUrl` constructs the callback URL from that endpoint and Core's
trusted callback origin. Missing mappings return `null`; successful writes are
not rolled back if a later script step fails.

`nextSequence(namespace)` returns a process-local Number starting at 1, scoped to
the current tenant, Space, Integration and connection. Reuse stable namespaces;
this is a request counter, not a durable or distributed ID. Counters reset when
the process restarts and fail rather than wrap at 899,999,999. Core bounds the
registry to 16,384 namespaces and each namespace to 1,024 UTF-8 bytes; saturation
does not evict live counters.

`readArtifact(image)` accepts an Image/URI in the current Space and returns a
`Uint8Array`, with an 8 MiB default read limit. Use it inside a script with a bundled
image decoder, then return computed business fields or an artifact URI. Binary
buffers are host/intermediate values, not Core execution-plan result values; use
an explicit number array when a small binary result must enter the engine. Core
results also require integers to fit signed 64-bit range. Never silently convert
an out-of-range BigInt to Number.

`putArtifact` accepts Uint8Array, `importArtifact` ingests an HTTP resource,
`resolveArtifact` obtains a bounded-lifetime delivery URL, and `deleteArtifact`
explicitly deletes a current-Space artifact. Identity/retention follow the existing
Artifact policy (`auto`/`man`, `std`/`tmp`). Completed writes survive a later script
failure. The current native put adapter translates bytes to the application
service's Base64 ingress DTO; JS values themselves do not use a JSON codec.

`openWebSocket`/`sendWebSocket`/`receiveWebSocket`/`closeWebSocket` and the matching
`TlsLine` helpers provide temporary invocation-owned connections. Use each handle
sequentially. WebSocket memory limits are fixed at open time; TLS lines have a
256 KiB ceiling. Local receive timeouts preserve partial reads for polling, while
the invocation deadline/cancellation releases its temporary sockets. Handles are
not valid in another invocation. These helpers do not manage persistent business
Channels or run detached receive loops.

```sh
npm ci
npm test
```

See the authoritative [shared contract](https://github.com/mhome-ai/foundation/blob/main/crates/script-api/contract/javascript-v1.md).

`browseMdns` exposes native local service discovery. `browseSsdp` sends IPv4 SSDP
M-SEARCH to the standard local multicast address; it accepts a search target, a
100–10,000 ms timeout and up to 64 responses, each at most 4 KiB. Its temporary
socket closes on completion or cancellation. Provider code parses response text; `requestGoogleCast` uses Core's
native Cast transport. Provider functions retain ownership of discovery identity,
capability mapping and error handling. Calls share the invocation deadline. Blocking
Cast IO uses the bounded native worker pool and retains admission until it actually
finishes; cancellation is best effort. `log` records bounded diagnostics attributed
to the invocation, using the existing Integration log directory.

`scanLocalTcp` uses Core's bounded local-network scanner; provider code chooses
ports and identifies devices from the results. It shares the invocation deadline
and does not expose arbitrary subnet selection.
