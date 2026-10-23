# Automation Script SDK and ABI v1

`mhome-script-sdk` is a host-independent Rust library compiled into each script.
It owns the constant-value codec, dynamic values, input-wrapper helpers, business
errors and guest memory ABI. `mhome-wasm-runtime` owns the process-wide Wasmtime
engine and Script invocation adapter. Neither crate depends on Core workflows,
storage, tenants, HTTP or a cloud implementation.

The Rust source is the authority for both Input and Output. The managed wrapper
imports the SDK prelude and includes the source; it does not infer Rust types from
YAML. The user declares named-field structs and a single entrypoint:

```rust
#[derive(ScriptInput)]
pub struct Input {
    pub target_lux: f64,
    pub samples: Vec<f64>,
    pub settings: Object,
}
#[derive(ScriptOutput)]
pub struct Output {
    pub brightness: f64,
    pub items: Vec<Object>,
}
pub fn run(input: Input) -> Result<Output, ScriptError> {
    Ok(Output { brightness: input.target_lux, items: vec![input.settings] })
}
```

The derives generate field metadata and conversions from the same definitions.
`mhome-script-sdk-macros` is re-exported through the SDK; the source only needs the
SDK prelude. The fixed wrapper invokes
`export_typed_script!(user_script::Input, user_script::Output, user_script::run)`.
Rust checks the entrypoint's parameter and result types during compilation.
[The typed wrapper fixture](examples/typed-lighting.rs) and
[its user source](examples/typed-lighting/source.rs) exercise this path.

Supported fields are `bool`, `f64`, `String`, `Temperature`, `Duration`,
`Timestamp`, `Image`, `Object`, `Value`, and recursively `Vec<T>`. Aliases and
qualified paths resolve through Rust traits. All fields are required, including
`Value`, which can contain an explicit Null. Missing, wrong-type and extra input
fields fail before `run`; errors identify the field and nested list indices.
`Object` accepts arbitrary nested fields, `Value` accepts any codec value, and
`Vec<T>` preserves every nesting level. A root business list is wrapped as a field,
for example `Output { items: Vec<Object> }`. Empty named structs (`Input {}`) work.

Tuple/unit structs, enums, generic roots, optional/default fields, serde
customization and nested typed business structs are not supported. Use dynamic
`Object` for nested business objects. No host IO or arbitrary dependency resolver
is provided. The older [lighting fixture](examples/lighting.rs) remains unchanged
for Core's existing dynamic ABI regression test; new scripts use the typed wrapper.

## Contract metadata

`script_contract() -> i64` exports metadata from the actual compiled Wasm. It uses
the same packed owned-buffer convention as invocation. Metadata is a plain
constant-codec Object (without the invocation result envelope):

```text
{
  version: 1,
  sdkVersion: "0.2.0",
  input:  [{ name: "samples", type: { list: "number" } }],
  output: [{ name: "items", type: { list: "object" } }]
}
```

Scalar type names are `bool`, `number`, `string`, `timestamp`, `duration`,
`temperature`, `image`, `object`, and `value`; `{ list: T }` nests recursively.
Input/output field arrays preserve Rust declaration order. Raw Rust identifiers
use their unprefixed name (`r#type` becomes `type`). Unknown contract versions,
type names, extra metadata properties, duplicate/empty/oversized field names and
malformed or over-budget codec data are rejected. `sdkVersion` records the SDK
that produced the module; a future build/install caller must compare it to the
requested pinned SDK. It is not an authenticity check.

`ScriptModule::contract` uses the existing shared engine and a fresh bounded
Store, validates the ABI, calls the metadata export, checks/copies the buffer and
frees it. It never calls `script_alloc`, `script_invoke` or business `run`.
Instantiation (including a Wasm start function), ABI query, metadata export and
deallocation all remain within the memory/time/cancellation limits. An old module
without the metadata export fails contract extraction; there is no Dynamic fallback.
YAML contract validation and the build service are later integration stages.

## Values

| Runtime value | SDK representation | Wire representation |
| --- | --- | --- |
| Null | `Value::Null` | tag `00` |
| Bool | `bool` | `02` / `03` |
| Number | `f64` | `04` + little-endian f64 |
| String | `String` | `01` + length + UTF-8 |
| Timestamp | `Timestamp { epoch_millis: i64 }` | `08` + i64 |
| Duration | `Duration { millis: i64, display_unit }` | `0a 01 unit` + i64 |
| Temperature | `Temperature { celsius: f64, display_unit }` | `0a 02 unit` + f32 |
| Other physical quantities | `Value::Quantity(Quantity)` | existing NumberUnit subtag, display unit, f32 |
| Date/time | `Value` calendar variants | existing `0c` / `0d` forms |
| Array | `Vec<T>` or `Value::Array` | `13` + count + values |
| Object | `BTreeMap<String, Value>` | `10` + count + key/value pairs |
| Image | `Image { uri: String }` | dynamic object with `uri` |

Numeric quantities use **base units**, regardless of the display unit: Celsius,
milliseconds, milligrams, millimeters, kilometers/hour. Use
`Temperature::from_celsius` and `Duration::from_millis` for common construction.
Display-unit bytes are the existing Core unit tags; invalid kind/unit combinations
are rejected. TemperatureDiff uses its existing distinct subtag and display units.
`LocalDateTime` remains unsupported by the existing constant wire format and is
rejected explicitly at the Core boundary.

Core Number values are converted to f64. Core floating-point milliseconds are
truncated to i64. Temperature and other floating physical quantities are narrowed
to f32 on the wire. These preserve the existing codec semantics. The SDK keeps
Duration/Timestamp integers exact within the guest; converting Duration back into
Core's f64 representation still has Core's numeric precision limit. No JSON,
Base64, or native Rust pointers cross the script boundary.

Image is a reference only in this version. Artifact reads, image decoding and
managed third-party libraries are not provided yet. Scripts have no host imports.
A URI in an object does not confer any access permission.

## Bounded constant codec

The low six tag bits identify a constant. For strings/objects/arrays, the upper
two bits select u8/u16/u32 length (`00`, `01`, `10`); `11` is reserved. Scalars
reject length bits. Existing string tag `12` is accepted when decoding. Numeric
payloads and lengths are little-endian. Object keys use one u8 UTF-8 byte length;
keys longer than 255 bytes fail instead of truncating. Keys encode in sorted order.

Defaults are 1 MiB encoded bytes, 64 nesting edges and 65,536 total values.
Encoding and decoding enforce the same budget. Invalid tags, UTF-8, quantities,
calendar values, duplicate keys, impossible lengths, empty payloads and trailing
bytes fail. Collection lengths are checked before allocating their contents.

`decode_prefix` returns the consumed length for Core instructions with multiple
operands. Script input/results always use strict, complete decoding. Core retains
its historical empty standalone constant -> Null compatibility only in its own
wrapper; guest input does not accept an empty buffer.

## Module exports and ownership

Wasm target: `wasm32-unknown-unknown`, unshared linear memory, no WASI or imports.
The typed wrapper emits `export_typed_script!(Input, Output, run)` and these exports:

| Export | Wasm signature | Responsibility |
| --- | --- | --- |
| `memory` | unshared wasm32 memory | guest address space |
| `script_abi_version` | `() -> i32` | must return 1 |
| `script_alloc` | `(i32 len) -> i32 ptr` | zero-initialized input allocation; zero means failure |
| `script_contract` | `() -> i64` | produce an owned metadata buffer; zero means failure |
| `script_invoke` | `(i32 ptr, i32 len) -> i64` | borrow input, produce a separate output allocation |
| `script_dealloc` | `(i32 ptr, i32 len) -> ()` | release exactly one allocation of that length |

The packed result uses pointer in the high 32 bits and length in the low 32 bits.
The response is `[1, status, constant...]`: status 0 is a successful dynamic value;
status 1 is an object containing exactly String `code` and String `message`.
A business error is distinct from a Wasmtime trap or invalid ABI/result.

The host allocates input through the guest, writes it and calls invoke. The guest
only borrows input and retains its newly allocated output. The host validates
output size, bounds and non-overlap **before** allocating/copying a host buffer,
then frees input and output once each. Every exit drops the per-call Store; on a
trap/invalid buffer it does not re-enter the failed guest to attempt cleanup.
Output conversion and both deallocations are part of the execution time budget.
SDK wrappers turn invalid input/output into structured errors; panics trap.

## Shared runtime policy

`mhome_wasm_runtime::engine()` is the sole production engine factory. Its clones
share one engine and one process-lifetime OS ticker. The ticker advances epoch
approximately every 10 ms independently of Tokio. Fuel is disabled. Each Store
checks its own cancellation token and elapsed-time budget and yields at an epoch
checkpoint. Instantiation, start, ABI query, allocation, invoke and deallocation
all use that policy. A Tokio timeout/select additionally covers asynchronous waits;
epoch yields make cancellation effective even on a single-thread executor.

Default Script limits: 5 seconds, 64 MiB linear memory, 10,000 table elements,
one instance/memory/table, and the codec limits above. Imports are rejected, and
no Integration linker capabilities are installed. Script compilation is bounded
to 16 MiB input and shares a two-slot blocking compilation budget with Integration.
A running compilation keeps its slot even if its waiter is dropped. Compilation
is not guest execution and does not use the invocation deadline.

Core Integration keeps its JSON ABI and its host linker. It shares the same engine
and compilation slots but creates its own Store. Integration keeps its existing
open-ended invocation policy; epoch yields allow its existing owner to cancel by
dropping the future. Every production Integration Store is configured **before**
instantiation. There is no global epoch increment used to cancel another Store.

Epoch cancellation is cooperative and is not a hard real-time guarantee. It does
not interrupt a blocking host function; future Script host IO must implement its
own cancellation and byte limits. Runtime iOS configuration retains Pulley,
disables signal traps and avoids large virtual-memory reservations.

## Validation

```sh
cargo fmt --all --check
cargo clippy -p mhome-script-sdk-macros -p mhome-script-sdk -p mhome-wasm-runtime --all-targets --locked -- -D warnings
bash scripts/test-script-wasm.sh
```

The script builds both real Rust fixtures for wasm32, runs codec/contract/runtime
tests and compile-fail doctests, and explicitly runs the fixture tests that normal
native-only `cargo test` skips.
CI installs the wasm32 target and runs this entrypoint. The runtime tests cover
loops/start functions, concurrent cancellation, dropped futures, memory/table
limits, invalid buffers/imports/ABI, typed conversion errors, metadata extraction
without business invocation, business errors and Pulley interruption.
An iOS-target compile and physical iOS execution are separate platform checks.

Core additionally tests its actual ConstValue conversion and existing Integration
host ABI. With the original `lighting` fixture built, set `SCRIPT_SDK_WASM` to its
absolute path and run Core's `rust_sdk_fixture` test with `-- --ignored` and the required `local-package`
profile. Source overrides used for local development are not published dependency
or installed system-E2E evidence.

## Product boundary

These libraries implement staged SDK and runtime support. Automation save/verify/
start still reject companion Rust sources until YAML contract checking, the build
worker, SCRIPT_CALL, per-Space module installation and activation are connected.
No module resolver, global script cache, cloud service or artifact permission
policy is invented here. The shared crates contain no Core-only execution flag.
