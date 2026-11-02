# Script authoring SDK

TypeScript authoring helpers for the JavaScript script migration. `defineScript`
declares Automation's named input/output fields once and infers the function's
TypeScript types. `t.object` and `t.value` preserve dynamic data; `t.list` preserves
element types and nesting. Integration entries use ordinary default-exported
functions and their role schemas instead of Automation contracts.

Functions return business data or throw `ScriptError`. This package does not
implement host IO, runtime validation, module sandboxing or the build-time static
contract extractor. Those are subsequent consumers of the shared contract. It is
private until the coordinated publication setup is ready.

```sh
npm ci
npm test
```

See the authoritative [shared contract](../../crates/script-api/contract/javascript-v1.md).
