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

Registry inputs support `.js`, `.mjs`, `.cjs`, `.ts`, `.mts`, `.cts` and `.json`;
other loaders, including JSX/TSX and native addons, are rejected. SDK imports always resolve to
the builder's exact managed SDK. Node built-ins, undeclared local files, symlinked
inputs and computed runtime imports are rejected. Project tsconfig files, build
plugins and lifecycle scripts are not loaded. Each entry is limited to 256 KiB,
each result to 4 MiB; compiler input is bounded to 4,096 files / 64 MiB, with no
single input over 4 MiB. The native host port is the only external runtime import.
These compiler APIs do not yet switch Integration staging, schema bindings or
Core's Integration executor to JavaScript; those consumers cut over together.

The distributable `@meowlink/script-build` npm package pins the SDK to an exact
version. Its `meowlink-script-build <project-directory> <functions-json-array>
<empty-output-directory>` command writes per-function JavaScript and `build.json`.
It does not install project dependencies or run their lifecycle scripts.

Source development keeps the SDK file link. Build `../script-sdk` first, then run
`npm ci --ignore-scripts` and `npm test` here. From the Foundation root, prepare
publishable directories with `node scripts/stage-script-package.mjs sdk <empty-dir>`
and `node scripts/stage-script-package.mjs build <empty-dir>`. These directories
contain executable/types artifacts, exact dependencies, README and LICENSE, with
no install hooks or sibling source paths. Pack with `npm pack <directory>`.
Tests install both tarballs offline and build a separate project without Foundation
sources. Staging/packing does not publish packages; registry publication and consumer
availability remain a separate release step.
