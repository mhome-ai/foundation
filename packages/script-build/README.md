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

`buildIntegration({projectRoot, functions})` checks a standalone function project
and returns one ESM artifact per selected `<function>.ts` entry. Shared source lives
under `common/`. Entries export a default function and retain ordered arguments;
Integration role schemas own business contracts, so these artifacts have no
Automation contract. The project has `package.json` and an npm v3
`package-lock.json`. Dev Kit callers must prepare dependencies with
`npm ci --ignore-scripts`; the compiler checks manifest/lock agreement and installed
versions and hashes the actual consumed source bytes. This check does not replace
npm's archive-integrity verification or reviewed build provenance.

Registry packages may contain pure JS/TS and JSON. SDK imports always resolve to
the builder's exact managed SDK. Node built-ins, undeclared local files, symlinked
inputs and computed runtime imports are rejected. Project tsconfig files, build
plugins and lifecycle scripts are not loaded. Each entry is limited to 256 KiB,
each result to 4 MiB; compiler input is bounded to 4,096 files / 64 MiB, with no
single input over 4 MiB. The native host port is the only external runtime import.
These compiler APIs do not yet switch Integration staging, schema bindings or
Core's Integration executor to JavaScript; those consumers cut over together.

During local development, build `../script-sdk` first, then `npm ci` and `npm test`
here. The local SDK dependency will become an exact published version during the
coordinated release cutover; this package is not currently publishable.
