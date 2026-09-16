# Topology bindings

Foundation owns Camera and Matter source DTOs, empty requests, targets and validation
in `crates/core-api/src/node/contracts/camera_topology.rs` and
`crates/core-api/src/node/contracts/matter_device_sources.rs`.
Camera sources use `/camera/device/sources`; Matter sources use `/matter/device/sources`.
Matter Node classifies bridges; Core must not interpret raw Fabric endpoints for this graph. The Device Topology
JSON schema owns the graph DTOs; `device-topology-relations.v1.json` owns relation
endpoint kinds, basis and Integration identity constraints.

Run from Foundation, with the consumer worktrees you intend to update:

```sh
node scripts/generate-topology-bindings.mjs --meowcore ../meowcore-rust --baycat ../baycat --lion-service ../lion/module-service --pallas ../pallas-cat
node scripts/generate-topology-bindings.mjs --meowcore ../meowcore-rust --baycat ../baycat --lion-service ../lion/module-service --pallas ../pallas-cat --check
node --test scripts/generate-topology-bindings.test.mjs
```

Outputs are committed generated source, not independently maintained DTOs or
runtime path dependencies. Camera bindings include their canonical tests. Java
records support all six entity kinds, including Hub forwarding; their constructors
validate fields and graph integrity. Rust and Java run the same generated graph
mutation corpus. This allows reproducible consumer builds without requiring a
package release for each binding update. Never edit generated outputs by hand.

Run `cargo test -p mhome-app-facade-api -p mhome-core-api --locked` and the consumer
topology tests after regeneration. `--check` rejects missing or stale bindings.

Core also consumes a generated Rust graph module and relation rules, sharing the
published common runtime/observation primitives. This keeps the graph revision
buildable without a protocol registry release or a sibling path dependency.
Canonical JSON Schema and fixtures are generated into Lion test resources so
its typed forwarding tests exercise the same revision. Do not change generated
bindings or test artifacts independently.

`matterBridge` replaces the former generic `sourceDevice` entity. Bridge identity
is scoped by connection; `nodeId` is the source identity, not the logical device ID.
Matter snapshots distinguish unavailable classification from a confirmed direct
node. This is a read-model/wire change, with no database migration. New Core needs
updated Lion validation when accessed over cloud relay; deploy relay readers before
new producers. Camera/Matter Node endpoint changes must accompany the Core update;
an older Node produces a partial graph without removing known devices.

The `--pallas` output pins the same graph schema, rules and fixtures under
`script/protocol/fixtures/` with source hashes. Frontend protocol generation uses
these inputs only for Device Topology until the registry package catches up;
all unrelated domains retain registry conformance checks. Run Pallas protocol
verification after refreshing the inputs.
