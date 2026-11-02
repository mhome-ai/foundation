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
