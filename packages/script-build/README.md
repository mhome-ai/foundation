# Script build

`buildAutomation` checks one TypeScript source against the exact Script SDK,
statically extracts its `defineScript` contract, and bundles ESM JavaScript. It
returns source/build identity and artifact SHA/size/contract metadata. It never
executes author source, installs packages, reads author tsconfig files, or allows
arbitrary filesystem imports. SDK helpers are bundled; the native host import stays
external. Runtime validates the actual values against the trusted contract.

Only literal SDK descriptors are accepted in input/output declarations. Ordinary
TypeScript is available in `run`. Import aliases are supported. Root lists use
named fields; nested objects stay dynamic. Errors carry bounded file/line/column
diagnostics. ES2022 output is source, never uploaded engine bytecode.

This package has no HTTP server, queue, authorization, or storage. The cloud build
adapter must isolate this CPU work in managed build workers and enforce its caller
deadline. An AbortSignal does not preempt TypeScript's synchronous checker.
Integration project/dependency bundling is a separate Dev Kit migration step.

During local development, build `../script-sdk` first, then `npm ci` and `npm test`
here. The local SDK dependency will become an exact published version during the
coordinated release cutover; this package is not currently publishable.
