# Script authoring SDK

TypeScript authoring helpers for the JavaScript script migration. `defineScript`
declares Automation's named input/output fields once and infers the function's
TypeScript types. `t.object` and `t.value` preserve dynamic data; `t.list` preserves
element types and nesting. Integration entries use ordinary default-exported
functions and their role schemas instead of Automation contracts.

Functions return business data or throw `ScriptError`. This package does not
implement privileged IO itself. `getRuntimeContext`, `sleep`, `randomU64` and
`readArtifact` call the native host module; that import remains external in built
ESM. Calling these helpers in ordinary Node author tooling fails explicitly.
The package remains private until coordinated publication and consumer cutover.

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

`readArtifact(image)` accepts an Image/URI in the current Space and returns a
`Uint8Array`, with an 8 MiB default read limit. Use it inside a script with a bundled
image decoder, then return computed business fields or an artifact URI. Binary
buffers are host/intermediate values, not Core execution-plan result values; use
an explicit number array when a small binary result must enter the engine. Core
results also require integers to fit signed 64-bit range. Never silently convert
an out-of-range BigInt to Number.

```sh
npm ci
npm test
```

See the authoritative [shared contract](../../crates/script-api/contract/javascript-v1.md).
