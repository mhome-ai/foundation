# JavaScript script contract v1

This is the accepted replacement contract for Integration functions and Automation
scripts. It describes the target; the presence of these models does not advertise
implemented Core or cloud execution. There is no Wasm compatibility mode.

## Ownership and execution

Authors write TypeScript. A managed build checks types, bundles dependencies and
emits ESM JavaScript source. Runtime loads source, never uploaded QuickJS bytecode.
Integration builds run in developer/CI tooling. Automation verification and saving
use the cloud TypeScript build service; installed local scripts run offline.

Core embeds native QuickJS-NG through rquickjs. Cloud uses the same Rust execution
library through an internal HTTP runtime service. The cloud build service uses
TypeScript tooling and does not require Rust compiler jobs or Docker per build.
The services share SDK/protocol versions but have independent admission budgets.

Integration source belongs to an immutable package version. Automation source
remains in AutomationScript.scripts; its artifacts belong to an Automation revision
and Space. Description-only changes do not increment the execution revision.

References carry kind, owner, version/revision, function and exact SHA-256. Tenant,
Space, operation, connection and permissions come from trusted invocation context.
A digest or a caller-provided owner never grants access. Resolvers validate ownership
before using a content cache. New artifacts use format `javascript-esm`.

## Authoring and contracts

Integration schema bindings use `script: {name, args}`. Args retain their ordered
meaning. Core-native `function:` bindings are separate. Entries export a default
function, synchronously or asynchronously return their canonical business data,
and throw ScriptError for a business failure. The runtime wraps internal outcomes;
authors do not return an outcome envelope. Integration role schemas remain the
business input/output authority.

Automation YAML retains `type: script`, a literal `function` identifier and named
`args.<field>` bindings. AutomationScript.scripts maps those identifiers to exact
TypeScript source strings, without paths or extensions. Every entry exports one
`defineScript({input, output, run})` from the platform SDK. Input and output are
literal named-field maps. Supported descriptors are `t.bool`, `t.number`,
`t.string`, `t.timestamp`, `t.duration`, `t.temperature`, `t.image`, `t.object`,
`t.value`, and recursively `t.list(descriptor)`.

The SDK infers TypeScript input/output types from these declarations. Build tools
statically read the bounded declaration syntax and emit ScriptContract metadata;
they do not execute the module to discover its types. Contract declarations may
not use computed keys, spreads or arbitrary expressions. Ordinary function bodies
remain TypeScript. Integration artifacts do not carry this Automation contract.

Every root field is required; `value` permits an explicit null value, not an absent
field. Top-level inputs/outputs are objects, and business lists use named fields.
Nested `object` values intentionally remain dynamic. Lists preserve nesting and
element types. YAML validation uses the emitted contract, including result field
visibility and existing variable/cycle rules. Runtime checks actual values against
the verified contract. No duplicate user-authored contract goes into YAML.

## Instruction

`SCRIPT_CALL` uses opcode **0x1d** with four operands:

1. Constant ScriptRef (Integration or Automation).
2. A normal value operand resolving to an argument array.
3. Success task ID.
4. Failure task ID.

Integration arguments are `[connect, request, ...]`. Automation arguments contain
exactly one input object: `[{field: value, ...}]`. Both call `entry(...args)`.
Success delivers business data to the success task; failure delivers ScriptError
to the failure task. Former opcode 0x13 and former Wasm opcode 0x1c are rejected,
not aliases. Other instruction semantics do not change. All plan generators,
cursors, interpreters and resource-reference collectors must cut over together.

## Values and host API

In-process calls convert Rust values to JS values directly, without JSON text.
Cross-process calls use a versioned typed value encoding; ordinary JSON numbers
must never silently round exact integers. The value implementation must preserve
the existing Core domain kinds, dates/times, quantity display units and fractional
duration precision. Integers outside JavaScript's safe Number range use BigInt;
explicit host u64 values use BigInt regardless of magnitude. Unsupported numeric
ranges, cycles, functions and invalid values fail with bounded diagnostics.
`t.number` means JavaScript Number; a value needing BigInt cannot silently satisfy
it and must use a dynamic `t.value` field. Binary host data uses Uint8Array. Across
processes, `ScriptValue` uses tagged values: i64/u64 payloads are canonical decimal
strings and bytes use standard padded Base64. This encoding never serializes a
large integer as a JSON number. Local calls do not take this serialization path.

The public SDK retains explicit domain objects: Temperature and Duration carry
`_type`, numeric `value` and `unit`, with values expressed in that unit. Temperature
unit identifiers are `C`, `F`, `K`; Duration uses `ms`, `sec`, `min`, `hour`, `day`,
matching Core's current JSON object vocabulary. Timestamp uses `_type: Timestamp`
and an integral epoch-millisecond `value` (Number when safe, otherwise BigInt).
Image remains an object containing its artifact `uri`. Preserve floating precision
when translating domain values; do not reuse the old f32 quantity encoding or
truncate fractional milliseconds. Other Core domain kinds remain part of dynamic
Value transport and must not be silently coerced to ordinary strings or numbers.

Calendar/recurring dates use `{_type: "DateGeneral", value: string}` and clock/solar
times use `{_type: "TimeGeneral", value: string}`. Solar offsets remain explicit,
including zero offsets, to retain their units. Local date-times use
`{_type: "LocalDateTime", value: ISO-local-string}` without timezone conversion.
Clock/local date-time values retain fractional seconds. Core engine results have
signed 64-bit integers and no binary-buffer variant: out-of-range u64 results and
Uint8Array results fail explicitly. Binary buffers remain usable between JS and
host APIs; return an artifact URI or an explicit number array to the engine.

Host APIs expose capabilities through platform modules. Pure helpers may be bundled;
privileged implementations are Rust/native bindings. No filesystem module loader,
Node process/require API or native dynamic module loader is provided. This is not
a complete Node or browser environment. Runtime context cannot be supplied or
replaced by script arguments. IO and return conversion have explicit byte/depth
limits in addition to the JS heap limit.

The SDK does not expose the former generic client-request capability (arbitrary
clientId plus target). Core's internal client communication remains internal.
Future client-facing capabilities require a concrete purpose and authorization
against the trusted invocation's tenant/Space; an arbitrary target is not a scope.

Core/Lion retain installation, connections, Flow, endpoints, subscriptions and
long-lived transports. Scripts are bounded functions and channel hooks, not service
owners. A channel request may invoke another script hook before the outer script
completes. Waiting for IO must not hold a global runtime lock or exhaust every slot
needed by those hooks. Each invocation has independent JS global state and heap.

The accepted security boundary is native, in-process JS with capability and resource
controls, not Wasmtime-equivalent memory isolation. No per-invocation container or
OS sandbox is required by this contract. This does not relax tenant/Space/resource
authorization or permit exposing platform credentials and arbitrary private APIs.

## Deadline and completion

All awaited work, including module initialization, Promise work, host IO and result
conversion, consumes the caller's deadline. Rust host futures and engine interrupts
observe cancellation. Cancellation is best effort; completed device effects are not
compensated. Release invocation-owned temporary resources. Do not tear down business
subscriptions or Core-owned long-lived transports as generic timeout cleanup.
No detached background API is exposed to scripts. Internal engine/host work cannot
silently acquire a fresh request budget. Resource limits apply before evaluating
module-level code. Native parsing also needs source/admission bounds; do not claim
interrupt callbacks can preempt every native operation.

## Storage and cutover

Build-generated contracts and JavaScript are bound by the same verified manifest.
Source/build identity includes the SDK, build profile and locked dependencies.
Resolve exact immutable versions, verify SHA/size, and retain artifacts referenced
by active plans/subscriptions and permitted prior revisions. Prepare resources before
committing AutomationData. Space migration transfers the matching artifacts and
validates destination availability before switching ownership.

Limits retain 100 Automation entries, 256 KiB YAML, 256 KiB source per entry and
4 MiB per compiled JS entry. There is no separate fixed 16 MiB Automation bundle
cap. Use bounded streaming and the existing large-payload transport instead of
holding every entry in memory or forcing all bytes into bridge messages.

Old Rust source, Wasm artifact schemas, guest ABIs, compiler containers and execution
branches are removed after consumer cutover. Regenerate execution plans and stage
new versions. Never overwrite immutable released content or silently rebuild a live
database on startup. Source changes, local tests and remote publication are separate.
