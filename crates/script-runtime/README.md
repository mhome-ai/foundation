# Script runtime

Runtime 0.2.0 loads only `@mhome/script/host`. Rebuild and republish older
scripts with the `@mhome/script` 0.2.0 toolchain before using this runtime.

Shared native QuickJS-NG execution through rquickjs. Core and the cloud script
service supply trusted host implementations and resolve/hash artifacts before
invocation. The library does not own storage, installation, transport, or Space
authorization. Each invocation owns a fresh runtime and JS heap.

Create one `ScriptWorkers` at service startup and share it across callers. It owns
dedicated OS threads with Tokio LocalSets, separate request/hook admission, and
fail-fast capacity limits. Waiting host IO yields to other invocations. A saturated
request pool leaves hook capacity available; recursive hook saturation fails instead
of waiting for its own caller. `shutdown` stops admission, interrupts running JS,
drops owned futures and waits for worker exit. Platform integration is separate work.

The lower-level `invoke` is a local future for dedicated executor adapters/tests.
Do not run it directly on a request executor: JavaScript executes synchronously
between awaits. Each invocation keeps its capacity until actual runtime teardown,
even if the caller has already received a timeout.

The caller supplies a mandatory absolute deadline and cancellation token. Limits
are applied before evaluating module code. Awaited host work shares the deadline;
dropping an invocation cancels its child token and drops its owned host futures.
Returning with pending host calls fails. Completed business effects are never
compensated by this library.

Only the platform host module can be imported at runtime. Other dependencies must
be bundled. Host implementations allowlist operation names, enforce caller scopes,
and bound IO. JS has no native filesystem/process/network APIs. This is in-process
execution, not Wasm-equivalent memory isolation or an OS sandbox. The interrupt
handler cannot promise immediate preemption of all native parser/builtin work.

Values convert directly between Rust and JS. Exact large integers use BigInt;
unsafe narrowing, nonfinite numbers, cycles, unsupported objects and undefined
fail. Binary values become Uint8Array and preserve slice offsets. Domain values
retain their object fields and display units. Cross-process typed serialization
belongs to `mhome-script-api`. Automation adapters validate arguments/results with
the verified ScriptContract; Integration adapters follow their role schemas.

```sh
cargo test --locked -p mhome-script-runtime
cargo clippy --locked -p mhome-script-runtime --all-targets -- -D warnings
```
